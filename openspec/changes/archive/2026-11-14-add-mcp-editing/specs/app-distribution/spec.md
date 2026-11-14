## MODIFIED Requirements

### Requirement: Per-user installer
The release SHALL include a Windows setup program. It SHALL install ripper for the current user only, into `%LOCALAPPDATA%\ripper`, without administrator rights or a UAC prompt. It SHALL place the MCP server `ripper-mcp.exe` next to `ripper.exe`, keep both updated together, and remove both on uninstall. It SHALL add a Start menu entry and register an uninstaller in Windows "Installed apps".

#### Scenario: Fresh install
- **WHEN** a user without administrator rights runs `ripper_1.0.0_x64-setup.exe` and accepts the defaults
- **THEN** `%LOCALAPPDATA%\ripper\ripper.exe` exists, a Start menu entry starts it, and no UAC prompt appeared

#### Scenario: MCP server installed
- **WHEN** the setup program of a version with the MCP server has run
- **THEN** `%LOCALAPPDATA%\ripper\ripper-mcp.exe` exists and has the same version as `ripper.exe`

#### Scenario: Uninstall
- **WHEN** the user removes ripper from Windows "Installed apps"
- **THEN** the program folder and the Start menu entry are gone
