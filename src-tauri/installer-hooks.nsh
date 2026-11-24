; Removes BloxClient's per-user Windows entries on a real uninstall. Updates
; also run the uninstaller, and must keep the startup preference.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "BloxClient"
    DeleteRegKey HKCU "Software\Classes\bloxclient"
  ${EndIf}
!macroend
