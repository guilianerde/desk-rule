Unicode true
!include "MUI2.nsh"

Name "Desk Rule"
OutFile "desk-rule-windows-x64-test-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Desk Rule"
RequestExecutionLevel user
Icon "res\icon.ico"
UninstallIcon "res\icon.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "${PAYLOAD_DIR}\desk-rule.exe"
  File "${PAYLOAD_DIR}\sciter.dll"
  File "${PAYLOAD_DIR}\LICENCE"
  File "${PAYLOAD_DIR}\README.md"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Desk Rule"
  CreateShortcut "$SMPROGRAMS\Desk Rule\Desk Rule.lnk" "$INSTDIR\desk-rule.exe"
  CreateShortcut "$SMPROGRAMS\Desk Rule\Uninstall Desk Rule.lnk" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DeskRule" "DisplayName" "Desk Rule"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DeskRule" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DeskRule" "DisplayIcon" "$INSTDIR\desk-rule.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DeskRule" "Publisher" "Desk Rule contributors"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DeskRule" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DeskRule" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\desk-rule.exe"
  Delete "$INSTDIR\sciter.dll"
  Delete "$INSTDIR\LICENCE"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Desk Rule\Desk Rule.lnk"
  Delete "$SMPROGRAMS\Desk Rule\Uninstall Desk Rule.lnk"
  RMDir "$SMPROGRAMS\Desk Rule"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\DeskRule"
SectionEnd
