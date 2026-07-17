# Dev code signing

VNKey currently builds unsigned (ad-hoc signature). This is fine for local
development, but a stable self-signed identity is needed once the app
starts requesting TCC-gated permissions (Accessibility, Input Monitoring —
see Task 8), because macOS ties those grants to a specific code signature
and an ad-hoc signature changes on every build.

## Create a self-signed "VNKey Dev" certificate

1. Open **Keychain Access** (`/Applications/Utilities/Keychain Access.app`).
2. Menu bar: **Keychain Access → Certificate Assistant → Create a Certificate…**
3. Fill in the dialog:
   - **Name:** `VNKey Dev`
   - **Identity Type:** Self Signed Root
   - **Certificate Type:** Code Signing
4. Check **Let me override defaults** if you want to set a longer validity
   period, otherwise the default is fine.
5. Click **Create**, then **Continue** through the remaining screens.
6. Find the new `VNKey Dev` certificate in the `login` keychain (under the
   **My Certificates** category), double-click it, expand **Trust**, and
   set **Code Signing** to **Always Trust**. Confirm with your password.
7. Verify it's usable for code signing:

   ```sh
   security find-identity -v -p codesigning
   ```

   You should see `VNKey Dev` listed with a valid (non-zero) identity count.

## Enable signing in tauri.conf.json

Once the certificate exists, add it to `src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "VNKey Dev"
    }
  }
}
```

`npx tauri build` will then sign the app bundle with this identity instead
of falling back to an ad-hoc signature.

## Migrating to a stable identity later

Task 8 (Accessibility / Input Monitoring permissions) will replace this
local self-signed certificate with a proper Developer ID (or an
organization-issued) signing identity, since TCC grants need to survive
distribution outside of this machine.
