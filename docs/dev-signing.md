# Dev code signing

VNKey builds unsigned (ad-hoc signature) by default, and `tauri.conf.json`
in this repo deliberately does **not** set `signingIdentity` - ad-hoc is
the intentional choice for the current stage of the project (see the M5
audit task notes). This is fine for a one-off build, but ad-hoc signatures
change on every rebuild, and macOS ties TCC-gated permission grants
(Accessibility, which VNKey requests to intercept/inject keys - see Task 8)
to a specific code signature. In practice that means: every time you
`cargo build`/`npx tauri build` again, macOS forgets the Accessibility
grant and you have to re-approve it in System Settings before VNKey can
type again.

If you're iterating locally and don't want to re-grant Accessibility after
every rebuild, create a stable self-signed identity below and point
`signingIdentity` at it **in your own local checkout** (do not commit that
change - the shipped config stays ad-hoc per the current product
decision).

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

This self-signed certificate only makes Accessibility grants survive
*your own local rebuilds*; it is not trusted by any other machine. If
VNKey is ever distributed outside of this machine (a DMG handed to other
users, notarization, etc.), it will need a proper Developer ID (or
organization-issued) signing identity instead - that is a distribution
decision for later, not something this repo configures today.
