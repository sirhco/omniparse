#!/usr/bin/env bash
#
# One-shot Cloud Run deploy for omniparse-web.
#
# Sets up:
#   1. A runtime service account (the identity the container runs as) with
#      Cloud Observability roles (logging.logWriter,
#      monitoring.metricWriter, cloudtrace.agent).
#   2. Deploys the image with --no-allow-unauthenticated so callers must
#      present a valid identity token issued for the service URL.
#   3. Optionally grants roles/run.invoker to a caller service account
#      passed as $3 (the workload that will call /parse and /detect).
#
# Usage:
#   bash deploy/cloud-run/deploy.sh <gcp-project> <region> [caller-sa-email]
#
# Examples:
#   bash deploy/cloud-run/deploy.sh my-proj us-central1
#   bash deploy/cloud-run/deploy.sh my-proj us-central1 \
#        client-app@my-proj.iam.gserviceaccount.com

set -euo pipefail

PROJECT="${1:?usage: deploy.sh <gcp-project> <region> [caller-sa-email]}"
REGION="${2:?usage: deploy.sh <gcp-project> <region> [caller-sa-email]}"
CALLER_SA="${3:-}"
IMAGE="${IMAGE:-ghcr.io/sirhco/omniparse-web:latest}"
RUNTIME_SA="omniparse-web@${PROJECT}.iam.gserviceaccount.com"

echo ">> Project:   $PROJECT"
echo ">> Region:    $REGION"
echo ">> Image:     $IMAGE"
echo ">> Runtime SA: $RUNTIME_SA"
[[ -n "$CALLER_SA" ]] && echo ">> Caller SA: $CALLER_SA"

# 1. Runtime SA. Idempotent: create-if-missing.
if ! gcloud iam service-accounts describe "$RUNTIME_SA" --project "$PROJECT" >/dev/null 2>&1; then
  echo ">> Creating runtime service account..."
  gcloud iam service-accounts create omniparse-web \
      --project "$PROJECT" \
      --display-name "Omniparse Web runtime"
fi

# Grant observability write roles.
for ROLE in roles/logging.logWriter \
            roles/monitoring.metricWriter \
            roles/cloudtrace.agent; do
  echo ">> Granting $ROLE to $RUNTIME_SA..."
  gcloud projects add-iam-policy-binding "$PROJECT" \
      --member "serviceAccount:${RUNTIME_SA}" \
      --role "$ROLE" \
      --condition=None \
      --quiet >/dev/null
done

# 2. Deploy.
echo ">> Deploying revision..."
gcloud run deploy omniparse-web \
  --project "$PROJECT" \
  --region "$REGION" \
  --image "$IMAGE" \
  --platform managed \
  --port 8080 \
  --concurrency 80 \
  --cpu 2 \
  --memory 2Gi \
  --timeout 120s \
  --min-instances 0 \
  --max-instances 10 \
  --no-cpu-throttling \
  --execution-environment gen2 \
  --no-allow-unauthenticated \
  --service-account "$RUNTIME_SA" \
  --set-env-vars "OMNIPARSE_OCR=ml,OMNIPARSE_LOG=info,OMNIPARSE_LOG_FORMAT=cloud,OMNIPARSE_PREWARM=1,OMNIPARSE_SHUTDOWN_GRACE_S=8"

# 3. Grant invoker to the caller SA, if supplied.
if [[ -n "$CALLER_SA" ]]; then
  echo ">> Granting roles/run.invoker to $CALLER_SA on omniparse-web..."
  gcloud run services add-iam-policy-binding omniparse-web \
      --project "$PROJECT" \
      --region "$REGION" \
      --member "serviceAccount:${CALLER_SA}" \
      --role "roles/run.invoker" \
      --quiet
fi

URL=$(gcloud run services describe omniparse-web \
        --project "$PROJECT" --region "$REGION" --format='value(status.url)')

cat <<EOF

Deployed: $URL

Smoke test (requires an IAM principal with roles/run.invoker):

    # From a machine with gcloud + the caller SA:
    TOKEN=\$(gcloud auth print-identity-token \\
        --impersonate-service-account=${CALLER_SA:-<CALLER_SA>} \\
        --audiences="$URL")

    curl -sf -H "Authorization: Bearer \$TOKEN" "$URL/live"
    curl -sf -H "Authorization: Bearer \$TOKEN" "$URL/ready"
    curl -sf -H "Authorization: Bearer \$TOKEN" \\
        -X POST -F file=@test_data/ocr/hello_world.png \\
        "$URL/parse" | jq .content

Inspect telemetry:
    gcloud logging read \\
        "resource.type=cloud_run_revision AND resource.labels.service_name=omniparse-web" \\
        --project "$PROJECT" --limit 5 --format json | jq '.[].jsonPayload'
    gcloud trace list-traces --project "$PROJECT" --limit 5

EOF
