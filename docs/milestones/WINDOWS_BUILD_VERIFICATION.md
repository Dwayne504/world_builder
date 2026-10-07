# Windows build verification

W02a provides a repeatable build procedure and a manually dispatched Windows CI job. It produces an unsigned native executable with a separate commit, version and checksum report. This is a development verification artifact, not a published installer or an automatic update channel.

## Build from the selected branch

Use a clean checkout containing the intended PRs, Node.js, an MSVC Rust toolchain, Visual Studio C++ build tools and the Windows SDK. From that repository root:

```powershell
.\scripts\Verify-WindowsBuild.ps1 -CheckOnly
.\scripts\Verify-WindowsBuild.ps1
```

The script stops on the first failed check, refuses uncommitted source changes, verifies all three app versions agree, and uses the committed dependency locks. It runs frontend and Rust checks before the production desktop build. Keep local work by using a separate worktree if necessary; do not reset it to satisfy the clean-checkout requirement.

The default output is `app/src-tauri/target/release/worldcrafter.exe`. An explicit `-TargetDirectory` can place build output elsewhere. The accompanying `windows-verification.json` identifies the source commit and executable SHA-256 so different branch builds can be distinguished without changing Project data. It does not claim byte-for-byte reproducible binaries.

After checking that the report identifies the intended commit, start the executable:

```powershell
Start-Process -FilePath .\app\src-tauri\target\release\worldcrafter.exe
```

This command intentionally opens the desktop application. Test with a newly created disposable Project, never by upgrading supplied originals. Help → About shows the running backend's supported Project format and schema. A Project needing a newer schema must be opened by a compatible build; do not edit the schema number or force an older build to accept it.

## Native acceptance still required

- Create, save, close and reopen a disposable Project; verify native folder selection and window-close save guards.
- Exercise both themes, keyboard navigation and Windows display scaling. Component tests do not prove WebView2 behavior.
- Use disposable older-version fixtures to verify upgrade, backup and Restore as Copy. Confirm a newer-version fixture is refused unchanged.
- Verify paths with spaces and non-ASCII names and behavior on a Windows account without development tools.
- Before a release, choose the installer/signing policy in roadmap decision D02 and test installation, upgrade and uninstall in an isolated Windows environment. Confirm user Projects and backups remain intact.

The manually dispatched **Windows build verification** workflow runs the same script and uploads only the executable and report. It does not launch the app, install it, publish a release or modify a user's Projects.
