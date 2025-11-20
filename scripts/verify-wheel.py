#!/usr/bin/env python3
"""
Script to verify Python wheel contents and metadata
"""
import sys
import zipfile
from pathlib import Path
import re


def verify_wheel(wheel_path):
    """Verify wheel contents and metadata"""
    print(f"Verifying wheel: {wheel_path}")
    print("=" * 80)
    
    if not Path(wheel_path).exists():
        print(f"Error: Wheel file not found: {wheel_path}")
        return False
    
    success = True
    
    with zipfile.ZipFile(wheel_path, 'r') as zf:
        files = zf.namelist()
        
        # Check required files
        required_files = [
            'omniparse/__init__.py',
            'omniparse/__init__.pyi',
            'omniparse/py.typed',
        ]
        
        print("\n1. Checking required files...")
        for req_file in required_files:
            if req_file in files:
                print(f"  ✓ {req_file}")
            else:
                print(f"  ✗ Missing: {req_file}")
                success = False
        
        # Check for native module
        print("\n2. Checking native module...")
        native_modules = [f for f in files if f.startswith('omniparse/_omniparse') and f.endswith('.so')]
        if native_modules:
            for mod in native_modules:
                print(f"  ✓ {mod}")
        else:
            print("  ✗ No native module found")
            success = False
        
        # Check metadata
        print("\n3. Checking metadata...")
        metadata_files = [f for f in files if 'METADATA' in f]
        if metadata_files:
            metadata_content = zf.read(metadata_files[0]).decode('utf-8')
            
            # Check version
            version_match = re.search(r'Version: (.+)', metadata_content)
            if version_match:
                print(f"  ✓ Version: {version_match.group(1)}")
            else:
                print("  ✗ Version not found")
                success = False
            
            # Check Python requirement
            python_match = re.search(r'Requires-Python: (.+)', metadata_content)
            if python_match:
                print(f"  ✓ Requires-Python: {python_match.group(1)}")
            else:
                print("  ✗ Python requirement not found")
                success = False
            
            # Check classifiers
            classifiers = re.findall(r'Classifier: (.+)', metadata_content)
            print(f"  ✓ Found {len(classifiers)} classifiers")
            
            # Check license
            license_match = re.search(r'License: (.+)', metadata_content)
            if license_match:
                print(f"  ✓ License: {license_match.group(1)}")
            else:
                print("  ✗ License not found")
                success = False
        else:
            print("  ✗ No metadata file found")
            success = False
        
        # Check wheel info
        print("\n4. Checking wheel info...")
        wheel_files = [f for f in files if 'WHEEL' in f]
        if wheel_files:
            wheel_content = zf.read(wheel_files[0]).decode('utf-8')
            print(f"  Wheel info:\n{wheel_content}")
        
        # List all files
        print("\n5. All files in wheel:")
        for f in sorted(files):
            print(f"  - {f}")
    
    print("\n" + "=" * 80)
    if success:
        print("✓ Wheel verification PASSED")
    else:
        print("✗ Wheel verification FAILED")
    
    return success


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: verify-wheel.py <wheel-file>")
        sys.exit(1)
    
    wheel_path = sys.argv[1]
    success = verify_wheel(wheel_path)
    sys.exit(0 if success else 1)
