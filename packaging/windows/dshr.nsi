; dshr Windows installer (draft).
;
; Build (Windows, NSIS installed):
;   makensis /DVERSION=0.1.0 /DSOURCE=target\release /DOUT=dist packaging/windows/dshr.nsi
;
; Per-user install in %LOCALAPPDATA%\dshr: no admin rights, no registry
; machine-wide writes, and `~/.dshr` user data is never touched by the
; uninstaller.

Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "WordFunc.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SOURCE
  !define SOURCE "target\release"
!endif
!ifndef OUT
  !define OUT "dist"
!endif

Name "dshr ${VERSION}"
OutFile "${OUT}\dshr-${VERSION}-setup.exe"
InstallDir "$LOCALAPPDATA\dshr"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\dshr-desktop.exe"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Russian"

Section "dshr (desktop + CLI)" SEC_MAIN
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${SOURCE}\dshr.exe"
  File "${SOURCE}\dshr-desktop.exe"

  CreateDirectory "$SMPROGRAMS\dshr"
  CreateShortCut "$SMPROGRAMS\dshr\dshr.lnk" "$INSTDIR\dshr-desktop.exe"
  CreateShortCut "$DESKTOP\dshr.lnk" "$INSTDIR\dshr-desktop.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\dshr" "DisplayName" "dshr"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\dshr" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\dshr" "Publisher" "dshr"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\dshr" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\dshr" "DisplayIcon" "$INSTDIR\dshr-desktop.exe"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\dshr" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\dshr" "NoRepair" 1
SectionEnd

Section "Add dshr to PATH (user)" SEC_PATH
  ; Append once; the uninstaller removes exactly this entry.
  ReadRegStr $0 HKCU "Environment" "Path"
  StrCmp $0 "" 0 +3
    WriteRegExpandStr HKCU "Environment" "Path" "$INSTDIR"
    Goto done
  ${StrContains} $1 "$INSTDIR" $0
  StrCmp $1 "" 0 done
    WriteRegExpandStr HKCU "Environment" "Path" "$0;$INSTDIR"
  done:
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\dshr.exe"
  Delete "$INSTDIR\dshr-desktop.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\dshr\dshr.lnk"
  RMDir "$SMPROGRAMS\dshr"
  Delete "$DESKTOP\dshr.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\dshr"

  ; Drop the PATH entry we added, if present.
  ReadRegStr $0 HKCU "Environment" "Path"
  ${StrContains} $1 "$INSTDIR" $0
  StrCmp $1 "" done
    ${WordReplace} "$0" ";$INSTDIR" "" "+" $0
    ${WordReplace} "$0" "$INSTDIR;" "" "+" $0
    ${WordReplace} "$0" "$INSTDIR" "" "+" $0
    WriteRegExpandStr HKCU "Environment" "Path" "$0"
  done:
  ; `%USERPROFILE%\.dshr` (sessions, config, keys) is intentionally kept.
SectionEnd

Function .onInit
  SetShellVarContext current
FunctionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_MAIN} "Copies dshr and dshr-desktop and registers an uninstaller."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_PATH} "Adds the install folder to the user PATH (for the dshr CLI)."
!insertmacro MUI_FUNCTION_DESCRIPTION_END
