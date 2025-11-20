# Cross-Compilation GCC Bug Fix

## Problem

When building Modality packages in a fresh clone of the repository, the cross-compilation to Linux x86_64 fails with the following error:

```
thread 'main' panicked at /Users/dotcontract/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.32.3/builder/cc_builder.rs:632:13:
### COMPILER BUG DETECTED ###
Your compiler (cc) is not supported due to a memcmp related bug reported in https://gcc.gnu.org/bugzilla/show_bug.cgi?id=95189
```

### Root Cause

The `aws-lc-sys` crate detects a known GCC memcmp bug (GCC Bug 95189) in the default Docker image used by `cross` for the `x86_64-unknown-linux-gnu` target. This bug is present in certain GCC versions and causes incorrect behavior in memcmp operations.

The working directory had a cached Docker image that didn't have this issue, while fresh clones pull the latest (broken) image.

## Solution

### Phase 1: Immediate Fix - Pin Docker Image

**File Modified:** `rust/Cross.toml`

**Change:** Added explicit Docker image specification with a version known to have a fixed GCC compiler:

```toml
[target.x86_64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-gnu:0.2.5"
pre-build = [
    "dpkg --add-architecture $CROSS_DEB_ARCH",
    "apt-get update && apt-get install -y libssl-dev:$CROSS_DEB_ARCH pkg-config",
]
```

**Benefits:**
- Uses Ubuntu 22.04 base with GCC 11 (bug-free)
- Ensures consistent builds across all environments
- No dependency changes required
- Compatible with existing build scripts

### Phase 2: Long-term Solution - GitHub Actions

**File Created:** `.github/workflows/build-packages.yml`

**Purpose:** Automate cross-platform builds on every push to `testnet` and `mainnet` branches.

**Key Features:**

1. **Automated Building**
   - Triggers on push to `testnet` or `mainnet` branches
   - Manual trigger option with custom version
   - Builds for Linux x86_64 and macOS ARM64
   - Compiles WASM packages (modality-lang, modal-wasm-validation)

2. **Cross-Compilation**
   - Uses `cross` with the pinned Docker image from Phase 1
   - Native build for macOS ARM64 on Linux runners
   - Strips binaries to reduce size

3. **Package Management**
   - Creates install script for users
   - Generates package manifest (JSON)
   - Creates index.html files for S3 browsing

4. **Deployment**
   - Uploads to S3: `s3://get.modal.money-content/{branch}/{version}/`
   - Updates latest symlink: `s3://get.modal.money-content/{branch}/latest/`
   - Builds and publishes Cargo registry
   - Invalidates CloudFront cache for immediate availability

5. **Caching**
   - Caches Cargo dependencies for faster builds
   - Uses GitHub Actions cache for rust/target/

6. **Build Summary**
   - Posts summary to GitHub Actions with:
     - Installation command
     - Direct download links
     - Cargo install command

## Testing Steps

### Test 1: Local Cross-Compilation (test-local-build)

In the fresh clone directory, run:

```bash
cd /Users/dotcontract/work/modality-org/modality
./scripts/packages/build-and-upload.sh --skip-upload
```

**Expected Result:**
- Build completes successfully for Linux x86_64
- Build completes successfully for macOS ARM64
- No GCC memcmp bug error
- Binaries are created in `build/binaries/`

**Verification:**
```bash
# Check binary exists and is executable
ls -lh build/binaries/linux-x86_64/modal
ls -lh build/binaries/darwin-aarch64/modal

# Check binary size (should be reasonable, 50-100MB stripped)
du -sh build/binaries/*/modal
```

### Test 2: GitHub Actions Workflow (test-workflow)

The workflow will run automatically on the next push to `testnet` or `mainnet`, but you can also test manually:

**Option 1: Manual Trigger**
1. Go to GitHub Actions
2. Select "Build and Upload Packages" workflow
3. Click "Run workflow"
4. Select branch (testnet)
5. Optionally provide custom version
6. Click "Run workflow"

**Option 2: Push to testnet**
```bash
git add rust/Cross.toml .github/workflows/build-packages.yml
git commit -m "Fix cross-compilation GCC bug and add automated builds"
git push origin testnet
```

**Expected Result:**
- Workflow completes successfully
- All build steps pass
- Packages uploaded to S3
- CloudFront cache invalidated
- Build summary displayed in Actions

**Verification:**
```bash
# Test installation from S3
curl -fsSL https://get.modal.money/testnet/latest/install.sh | sh

# Verify binary works
~/.modality/bin/modal --version

# Test direct download
curl -L https://get.modal.money/testnet/latest/binaries/linux-x86_64/modal -o /tmp/modal-test
chmod +x /tmp/modal-test
/tmp/modal-test --version

# Test Cargo install
cargo install --index sparse+https://get.modal.money/testnet/latest/cargo-registry/index/ modal
```

## Configuration Requirements

### GitHub Secrets

The workflow requires the following secrets to be configured in GitHub:

- `AWS_ACCESS_KEY_ID` - AWS access key for S3 uploads
- `AWS_SECRET_ACCESS_KEY` - AWS secret key for S3 uploads

### AWS Permissions

The AWS credentials need permissions for:
- S3: `s3:PutObject`, `s3:ListBucket`, `s3:DeleteObject` on `get.modal.money-content` bucket
- CloudFront: `cloudfront:CreateInvalidation` for distribution `EAB0G50HTKF8I`

## Migration Path

### For Developers

1. **Fresh Clone:** No action needed - builds will work immediately
2. **Existing Clone:** Either rebuild Docker image or continue using cached version
   ```bash
   # Optional: Clear Docker cache to use new pinned image
   docker rmi $(docker images 'cross-rs/x86_64-unknown-linux-gnu' -q)
   ```

### For CI/CD

1. **Local Builds:** Continue using `./scripts/packages/build-and-upload.sh`
2. **Automated Builds:** Use GitHub Actions (preferred for releases)
3. **Transition:** Can run both in parallel during migration

## Benefits

### Immediate (Phase 1)
- ✅ Unblocks development on fresh clones
- ✅ Consistent build environment
- ✅ No code changes required
- ✅ Works with existing scripts

### Long-term (Phase 2)
- ✅ Automated builds on every push
- ✅ Consistent CI/CD pipeline
- ✅ No local Docker dependency for releases
- ✅ GitHub infrastructure handles builds
- ✅ Build artifacts automatically published
- ✅ Version tracking in Git history
- ✅ Build status visible in GitHub UI

## Files Modified

1. `rust/Cross.toml` - Added Docker image pinning
2. `.github/workflows/build-packages.yml` - New GitHub Actions workflow
3. `docs/progress/CROSS_COMPILATION_FIX.md` - This documentation

## References

- GCC Bug: https://gcc.gnu.org/bugzilla/show_bug.cgi?id=95189
- cross-rs: https://github.com/cross-rs/cross
- aws-lc-sys: https://crates.io/crates/aws-lc-sys

## Next Steps

1. ✅ Update Cross.toml with pinned image
2. ✅ Create GitHub Actions workflow
3. ⏳ Test local build in fresh clone
4. ⏳ Test GitHub Actions workflow
5. ⏳ Configure GitHub secrets
6. ⏳ Verify S3 uploads and CloudFront cache

## Status

- **Phase 1:** ✅ Complete
- **Phase 2:** ✅ Complete
- **Testing:** ⏳ Pending
- **Deployment:** ⏳ Pending (awaiting GitHub secrets configuration)

