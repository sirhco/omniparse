//! Confidence scoring for type detection

use super::detector::DetectionMethod;

/// Calculate confidence score based on detection method
/// 
/// Returns a confidence score between 0.0 and 1.0:
/// - Magic bytes: 0.9-1.0 (very high confidence)
/// - Content analysis: 0.6-0.8 (medium-high confidence)
/// - Extension only: 0.3-0.5 (low-medium confidence)
/// - Unknown: 0.0-0.2 (very low confidence)
pub fn calculate_confidence(method: DetectionMethod) -> f32 {
    match method {
        DetectionMethod::MagicBytes => 0.95,
        DetectionMethod::ContentAnalysis => 0.7,
        DetectionMethod::Extension => 0.4,
        DetectionMethod::Unknown => 0.1,
    }
}

/// Calculate confidence with additional context
/// 
/// This function allows for more nuanced confidence scoring based on
/// multiple factors beyond just the detection method.
pub fn calculate_confidence_with_context(
    method: DetectionMethod,
    has_multiple_indicators: bool,
    data_size: usize,
) -> f32 {
    let base_confidence = calculate_confidence(method);
    
    // Boost confidence if multiple indicators agree
    let indicator_boost = if has_multiple_indicators { 0.05 } else { 0.0 };
    
    // Reduce confidence for very small files (less reliable)
    let size_penalty = if data_size < 10 {
        -0.1
    } else if data_size < 100 {
        -0.05
    } else {
        0.0
    };
    
    // Clamp to valid range [0.0, 1.0]
    (base_confidence + indicator_boost + size_penalty).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_magic_bytes_confidence() {
        let confidence = calculate_confidence(DetectionMethod::MagicBytes);
        assert!(confidence >= 0.9 && confidence <= 1.0);
    }
    
    #[test]
    fn test_content_analysis_confidence() {
        let confidence = calculate_confidence(DetectionMethod::ContentAnalysis);
        assert!(confidence >= 0.6 && confidence <= 0.8);
    }
    
    #[test]
    fn test_extension_confidence() {
        let confidence = calculate_confidence(DetectionMethod::Extension);
        assert!(confidence >= 0.3 && confidence <= 0.5);
    }
    
    #[test]
    fn test_unknown_confidence() {
        let confidence = calculate_confidence(DetectionMethod::Unknown);
        assert!(confidence >= 0.0 && confidence <= 0.2);
    }
    
    #[test]
    fn test_confidence_with_context() {
        // Test with multiple indicators
        let confidence = calculate_confidence_with_context(
            DetectionMethod::ContentAnalysis,
            true,
            1000,
        );
        assert!(confidence > calculate_confidence(DetectionMethod::ContentAnalysis));
        
        // Test with small file size
        let confidence = calculate_confidence_with_context(
            DetectionMethod::MagicBytes,
            false,
            5,
        );
        assert!(confidence < calculate_confidence(DetectionMethod::MagicBytes));
    }
    
    #[test]
    fn test_confidence_clamping() {
        // Ensure confidence never exceeds 1.0
        let confidence = calculate_confidence_with_context(
            DetectionMethod::MagicBytes,
            true,
            10000,
        );
        assert!(confidence <= 1.0);
        
        // Ensure confidence never goes below 0.0
        let confidence = calculate_confidence_with_context(
            DetectionMethod::Unknown,
            false,
            1,
        );
        assert!(confidence >= 0.0);
    }
}
