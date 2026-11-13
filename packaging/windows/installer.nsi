; tracewing's installer for Windows (NSIS 3, Unicode), made by
; scripts/package_windows.sh, as packwing's is:
;
;   makensis /DVERSION=0.1.0 /DFILEVERSION=0.1.0.0 /DEXE=path\to\tracewing.exe
;            /DLICENSES=path\to\THIRD-PARTY-LICENSES /DOUT=setup.exe installer.nsi
;
; FILEVERSION is VERSION as Windows writes a file's version: four numbers, no
; beta (0.3.0-beta2 is 0.3.0.0).
;
; It installs for everyone on the computer, into Program Files, where only an
; administrator changes it; or for the user alone, with no administrator
; rights, into %LOCALAPPDATA%\Programs\tracewing. A page asks which, for
; everyone by default; /ALLUSERS and /CURRENTUSER say it on the command line,
; as /S installs quietly. Where tracewing is installed already, it goes there
; again, as it was: to change, uninstall it first.
;
; It runs as the user. For everyone, it starts itself again as an
; administrator (Windows asks), waits for that one, and ends. The app it
; starts at the end is the user's, never the administrator's.
;
; One file is the program: the window, the proxy, the command line and the MCP
; server are all tracewing.exe. Its folder goes in the PATH (path.ps1), the
; user's or, for everyone, the computer's, so a terminal opened after runs
; `tracewing capture`; uninstalling takes it out again.
;
; The settings and what was recorded are each user's, under %LOCALAPPDATA%:
; neither installing nor uninstalling touches them. Uninstalling does run
; `tracewing uninstall`, which takes the certificate authority out of the
; stores that trust it and puts the system proxy back — the two things
; tracewing changes outside its own folder.

Unicode true

SetCompressor /SOLID lzma

!include "MUI2.nsh"
!include "WinMessages.nsh"
!include "LogicLib.nsh"
!include "nsDialogs.nsh"
!include "FileFunc.nsh"

!ifndef VERSION
  !error "VERSION is not given"
!endif
!ifndef FILEVERSION
  !error "FILEVERSION is not given"
!endif
!ifndef EXE
  !error "EXE is not given"
!endif
!ifndef LICENSES
  !error "LICENSES is not given"
!endif
!ifndef OUT
  !error "OUT is not given"
!endif

!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\tracewing"

; path.ps1, with the folder as it is, whatever is in its name.
!define PATH_PS1 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\path.ps1"'

Name "tracewing"
OutFile "${OUT}"
; Where, as the mode says: .onInit and the mode's page set it.
InstallDir ""
RequestExecutionLevel user
BrandingText "tracewing ${VERSION}"

; 1 for everyone, 0 for the user alone.
Var Everyone
; 1 once the mode is known (said, or tracewing installed already): no page.
Var Known
; 1 when /D= said where.
Var Told
; 1 in the installer started again as an administrator.
Var Elevated
Var ForEveryone
Var ForMe

VIProductVersion "${FILEVERSION}"
VIAddVersionKey "ProductName" "tracewing"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "tracewing installer"
VIAddVersionKey "CompanyName" "HivePaaS"
VIAddVersionKey "LegalCopyright" "Copyright (C) 2026 HivePaaS"

!define MUI_ICON "..\icons\tracewing.ico"
!define MUI_UNICON "..\icons\tracewing.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "Start tracewing"
!define MUI_FINISHPAGE_RUN_FUNCTION StartApp

!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipWhenElevated
!insertmacro MUI_PAGE_WELCOME
Page custom ModePage ModeLeave
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; The installer's 32 bits would see the registry's 32-bit view: tracewing runs
; on 64-bit Windows alone, where the keys are in the other.
!macro VIEW
  SetRegView 64
!macroend

; Where, and in whose name, as $Everyone says.
!macro MODE
  ${If} $Everyone == 1
    SetShellVarContext all
    ${If} $Told != 1
      StrCpy $INSTDIR "$PROGRAMFILES64\tracewing"
    ${EndIf}
  ${Else}
    SetShellVarContext current
    ${If} $Told != 1
      StrCpy $INSTDIR "$LOCALAPPDATA\Programs\tracewing"
    ${EndIf}
  ${EndIf}
!macroend

; Whether this runs as an administrator: the user's token, under UAC, says
; "User" until Windows raises it.
!macro IS_ADMIN out
  UserInfo::GetAccountType
  Pop ${out}
!macroend

Function .onInit
  !insertmacro VIEW
  ${If} $INSTDIR != ""
    StrCpy $Told 1
  ${EndIf}
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/ELEVATED" $R1
  ${IfNot} ${Errors}
    StrCpy $Elevated 1
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/ALLUSERS" $R1
  ${IfNot} ${Errors}
    StrCpy $Everyone 1
    StrCpy $Known 1
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/CURRENTUSER" $R1
  ${IfNot} ${Errors}
    StrCpy $Everyone 0
    StrCpy $Known 1
  ${EndIf}
  ; Installed already: there again, as it was.
  ${If} $Known != 1
    ReadRegStr $R1 HKLM "${UNINSTALL_KEY}" "InstallLocation"
    ReadRegStr $R2 HKCU "${UNINSTALL_KEY}" "InstallLocation"
    ${If} $R1 != ""
      StrCpy $Everyone 1
      StrCpy $Known 1
    ${ElseIf} $R2 != ""
      StrCpy $Everyone 0
      StrCpy $Known 1
    ${Else}
      StrCpy $Everyone 1
    ${EndIf}
  ${EndIf}
  ; Quiet, nothing is asked: for everyone where nothing says otherwise.
  ${If} ${Silent}
    StrCpy $Known 1
  ${EndIf}
  ${If} $Known == 1
  ${AndIf} $Everyone == 1
    Call Elevate
  ${EndIf}
  !insertmacro MODE
FunctionEnd

; For everyone, as an administrator: this installer started again so, with
; what it was told, and waited for; then this one ends.
Function Elevate
  !insertmacro IS_ADMIN $0
  ${If} $0 == "Admin"
    Return
  ${EndIf}
  ${If} $Elevated == 1
    MessageBox MB_ICONSTOP "tracewing could not be installed for everyone: it needs an administrator's rights." /SD IDOK
    Quit
  ${EndIf}
  ${GetParameters} $R0
  ; /D= comes last, as NSIS reads it.
  ClearErrors
  ExecShellWait "runas" "$EXEPATH" '/ELEVATED /ALLUSERS $R0'
  ${If} ${Errors}
    MessageBox MB_ICONEXCLAMATION "tracewing was not installed: Windows did not give it an administrator's rights. Install it for yourself alone to do without them." /SD IDOK
  ${EndIf}
  Quit
FunctionEnd

Function SkipWhenElevated
  ${If} $Elevated == 1
    Abort
  ${EndIf}
FunctionEnd

Function ModePage
  ${If} $Known == 1
    Abort
  ${EndIf}
  !insertmacro MUI_HEADER_TEXT "Install for whom?" "tracewing can be installed for everyone on this computer, or for you alone."
  nsDialogs::Create 1018
  Pop $0
  ${NSD_CreateRadioButton} 0 0 100% 12u "For everyone on this computer (recommended)"
  Pop $ForEveryone
  ${NSD_CreateLabel} 12u 14u -12u 34u "Into Program Files, where only an administrator can change it. Windows asks for an administrator's permission now, and for each new version."
  Pop $0
  ${NSD_CreateRadioButton} 0 56u 100% 12u "Only for me"
  Pop $ForMe
  ${NSD_CreateLabel} 12u 70u -12u 34u "Into your own folder, with no administrator's permission."
  Pop $0
  ${If} $Everyone == 1
    ${NSD_Check} $ForEveryone
  ${Else}
    ${NSD_Check} $ForMe
  ${EndIf}
  nsDialogs::Show
FunctionEnd

Function ModeLeave
  ${NSD_GetState} $ForEveryone $0
  ${If} $0 == ${BST_CHECKED}
    StrCpy $Everyone 1
    Call Elevate
  ${Else}
    StrCpy $Everyone 0
  ${EndIf}
  !insertmacro MODE
FunctionEnd

; The app, started as the user who is signed in, never as the administrator
; this may run as: Explorer starts it.
Function StartApp
  Exec '"$WINDIR\explorer.exe" "$INSTDIR\tracewing.exe"'
FunctionEnd

Section "tracewing"
  ; tracewing may still be running: the new one is unpacked aside, then copied
  ; in, tried again for five seconds, then asked about (Retry / Cancel; Cancel
  ; when quiet).
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File "/oname=tracewing.exe" "${EXE}"
  SetOutPath "$INSTDIR"
  StrCpy $0 0
  retry:
    ClearErrors
    CopyFiles /SILENT "$PLUGINSDIR\tracewing.exe" "$INSTDIR\tracewing.exe"
    IfErrors 0 copied
    IntOp $0 $0 + 1
    IntCmp $0 10 ask
    Sleep 500
    Goto retry
  ask:
    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "tracewing is still running. Quit it, then choose Retry." /SD IDCANCEL IDRETRY retry
    Abort
  copied:

  File "/oname=LICENSE.txt" "..\..\LICENSE"
  File "/oname=THIRD-PARTY-LICENSES.txt" "${LICENSES}"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\tracewing.lnk" "$INSTDIR\tracewing.exe"

  ; The folder in the PATH, once: tracewing.exe is the command line too.
  SetOutPath "$PLUGINSDIR"
  File "path.ps1"
  SetOutPath "$INSTDIR"
  DetailPrint "Putting tracewing in the PATH"
  ${If} $Everyone == 1
    nsExec::ExecToLog '${PATH_PS1} add "$INSTDIR" machine'
  ${Else}
    nsExec::ExecToLog '${PATH_PS1} add "$INSTDIR" user'
  ${EndIf}
  Pop $0
  StrCmp $0 "0" +2
    DetailPrint "The PATH was not changed: tracewing is in $INSTDIR"
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000

  ; In the user's, or for everyone the computer's (SHCTX, as the mode set it).
  WriteRegStr SHCTX "${UNINSTALL_KEY}" "DisplayName" "tracewing"
  WriteRegStr SHCTX "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr SHCTX "${UNINSTALL_KEY}" "Publisher" "HivePaaS"
  WriteRegStr SHCTX "${UNINSTALL_KEY}" "URLInfoAbout" "https://tracewing.app"
  WriteRegStr SHCTX "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\tracewing.exe"
  WriteRegStr SHCTX "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr SHCTX "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr SHCTX "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD SHCTX "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD SHCTX "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

; The uninstaller knows the mode from where it is: the computer's key names
; its folder for an install for everyone, which it removes as an
; administrator, started again so as the installer is.
Function un.onInit
  !insertmacro VIEW
  ReadRegStr $R1 HKLM "${UNINSTALL_KEY}" "InstallLocation"
  ${If} $R1 == $INSTDIR
    StrCpy $Everyone 1
    !insertmacro IS_ADMIN $0
    ${If} $0 != "Admin"
      ${GetParameters} $R0
      ClearErrors
      ${GetOptions} $R0 "/ELEVATED" $R2
      ${IfNot} ${Errors}
        MessageBox MB_ICONSTOP "tracewing could not be uninstalled: it needs an administrator's rights." /SD IDOK
        Quit
      ${EndIf}
      ClearErrors
      ExecShellWait "runas" "$EXEPATH" '/ELEVATED $R0'
      ${If} ${Errors}
        MessageBox MB_ICONEXCLAMATION "tracewing was not uninstalled: Windows did not give it an administrator's rights." /SD IDOK
      ${EndIf}
      Quit
    ${EndIf}
    SetShellVarContext all
  ${Else}
    StrCpy $Everyone 0
    SetShellVarContext current
  ${EndIf}
FunctionEnd

Section "Uninstall"
  ; The two things tracewing changes outside its own folder: the certificate
  ; authority this computer was told to trust, and the system proxy. It runs
  ; as whoever is uninstalling, so it is that user's trust store and that
  ; user's settings — which is the user who installed it, bar an
  ; administrator uninstalling somebody else's.
  nsExec::ExecToLog '"$INSTDIR\tracewing.exe" uninstall'
  Pop $0
  StrCmp $0 "0" +2
    DetailPrint "tracewing's certificate authority may still be trusted: open tracewing's Set up page before removing it, or remove it in certmgr.msc"

  ; The folder out of the PATH.
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File "path.ps1"
  ${If} $Everyone == 1
    nsExec::ExecToLog '${PATH_PS1} remove "$INSTDIR" machine'
  ${Else}
    nsExec::ExecToLog '${PATH_PS1} remove "$INSTDIR" user'
  ${EndIf}
  Pop $0
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
  ; Out of the folder, which is taken away below.
  SetOutPath "$TEMP"

  Delete "$INSTDIR\tracewing.exe"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\THIRD-PARTY-LICENSES.txt"
  Delete "$INSTDIR\uninstall.exe"
  RMDir /REBOOTOK "$INSTDIR"
  Delete "$SMPROGRAMS\tracewing.lnk"
  DeleteRegKey SHCTX "${UNINSTALL_KEY}"
SectionEnd
