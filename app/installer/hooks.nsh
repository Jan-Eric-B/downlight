; Downlight's additions to Tauri's NSIS installer (ticket 17), included from tauri.conf.json's
; bundle.windows.nsis.installerHooks. The installer runs per user, so SHCTX is HKCU and nothing
; here needs admin rights.
;
; File associations: Downlight registers as one of the programs for each Markdown extension, the
; way Microsoft asks (an app-specific ProgID, OpenWithProgids for "Open with", Capabilities and
; RegisteredApplications for Default apps), and leaves choosing the default to the reader. Tauri's
; own bundle.fileAssociations isn't used: it overwrites each extension's default ProgID and writes
; no OpenWithProgids or Capabilities, so Downlight wouldn't appear in Default apps, and on
; uninstall it writes the old default back over whatever the reader chose since and leaves its
; backup value behind.
;
; PATH: the install folder goes at the end of the user's PATH and comes out again on uninstall.
; NSIS strings hold NSIS_MAX_STRLEN characters and ReadRegStr gives an empty string for a longer
; value, so a PATH it can't hold whole is left alone and the reader is told, rather than cut short
; or overwritten.
;
; This file is included before Tauri's own defines and variables, so only the hook macros, which
; are expanded later inside Tauri's sections, use them (MANUPRODUCTKEY, MAINBINARYNAME,
; PRODUCTNAME, UpdateMode).

!include LogicLib.nsh
!include WinMessages.nsh

!define DOWNLIGHT_PROGID "Downlight.Markdown"
!define DOWNLIGHT_ENVIRONMENT "Environment"

; The extensions the core opens as Markdown (MARKDOWN_EXTENSIONS in core/src/format.rs).
; app/tests/installer.rs checks that the two lists agree.
!macro DOWNLIGHT_FOR_EACH_EXTENSION MACRO
  !insertmacro ${MACRO} "md"
  !insertmacro ${MACRO} "markdown"
  !insertmacro ${MACRO} "mdown"
  !insertmacro ${MACRO} "mkd"
  !insertmacro ${MACRO} "mkdn"
  !insertmacro ${MACRO} "mdwn"
!macroend

!macro DOWNLIGHT_ASSOCIATE EXTENSION
  WriteRegStr SHCTX "Software\Classes\.${EXTENSION}\OpenWithProgids" "${DOWNLIGHT_PROGID}" ""
  WriteRegStr SHCTX "${MANUPRODUCTKEY}\Capabilities\FileAssociations" ".${EXTENSION}" "${DOWNLIGHT_PROGID}"
!macroend

; Removes only Downlight's own value, then the keys that are left with no values and no subkeys,
; so another program's registration for the extension stays as it was.
!macro DOWNLIGHT_UNASSOCIATE EXTENSION
  DeleteRegValue SHCTX "Software\Classes\.${EXTENSION}\OpenWithProgids" "${DOWNLIGHT_PROGID}"
  DeleteRegKey /ifempty SHCTX "Software\Classes\.${EXTENSION}\OpenWithProgids"
  DeleteRegKey /ifempty SHCTX "Software\Classes\.${EXTENSION}"
!macroend

; What DownlightReadPath and DownlightLeaveOutInstallFolder hand on: the PATH as read, the PATH
; without the install folder, and whether it held it (1 or 0).
Var DownlightPath
Var DownlightPathWithout
Var DownlightPathHadIt

!macro DOWNLIGHT_PATH_FUNCTIONS UN
  ; Reads the user's PATH into $DownlightPath and leaves "ok", "none" (there is no PATH value) or
  ; "unsafe" (a value NSIS can't hold whole, or not a string) on the stack.
  Function ${UN}DownlightReadPath
    Push $R0
    Push $R1
    ClearErrors
    ReadRegStr $DownlightPath HKCU "${DOWNLIGHT_ENVIRONMENT}" "Path"
    ${If} ${Errors}
      ; ReadRegStr fails the same way for a missing value and a too long one, so look for it.
      StrCpy $R0 "none"
      StrCpy $R1 0
      ${Do}
        ClearErrors
        EnumRegValue $DownlightPath HKCU "${DOWNLIGHT_ENVIRONMENT}" $R1
        ${IfThen} ${Errors} ${|} ${ExitDo} ${|}
        ${If} $DownlightPath == "Path"
          StrCpy $R0 "unsafe"
          ${ExitDo}
        ${EndIf}
        IntOp $R1 $R1 + 1
      ${Loop}
      StrCpy $DownlightPath ""
    ${Else}
      ; A value that fills the whole buffer may have been cut at its end.
      StrLen $R1 $DownlightPath
      IntOp $R1 $R1 + 1
      ${If} $R1 >= ${NSIS_MAX_STRLEN}
        StrCpy $R0 "unsafe"
      ${Else}
        StrCpy $R0 "ok"
      ${EndIf}
    ${EndIf}
    Pop $R1
    Exch $R0
  FunctionEnd

  ; Splits $DownlightPath at its semicolons and joins it again into $DownlightPathWithout without
  ; the entries that name the install folder (ignoring case, with or without a trailing
  ; backslash). Every other entry, an empty one too, stays as it was.
  Function ${UN}DownlightLeaveOutInstallFolder
    Push $R0 ; position
    Push $R1 ; length
    Push $R2 ; character
    Push $R3 ; entry
    Push $R4 ; whether an entry has been kept yet
    StrCpy $DownlightPathWithout ""
    StrCpy $DownlightPathHadIt 0
    StrCpy $R0 0
    StrLen $R1 $DownlightPath
    StrCpy $R3 ""
    StrCpy $R4 0
    ${Do}
      StrCpy $R2 $DownlightPath 1 $R0
      ${If} $R0 >= $R1
      ${OrIf} $R2 == ";"
        ${If} $R3 == "$INSTDIR"
        ${OrIf} $R3 == "$INSTDIR\"
          StrCpy $DownlightPathHadIt 1
        ${ElseIf} $R4 = 0
          StrCpy $DownlightPathWithout $R3
          StrCpy $R4 1
        ${Else}
          StrCpy $DownlightPathWithout "$DownlightPathWithout;$R3"
        ${EndIf}
        StrCpy $R3 ""
        ${IfThen} $R0 >= $R1 ${|} ${ExitDo} ${|}
      ${Else}
        StrCpy $R3 "$R3$R2"
      ${EndIf}
      IntOp $R0 $R0 + 1
    ${Loop}
    Pop $R4
    Pop $R3
    Pop $R2
    Pop $R1
    Pop $R0
  FunctionEnd
!macroend

!insertmacro DOWNLIGHT_PATH_FUNCTIONS ""
!insertmacro DOWNLIGHT_PATH_FUNCTIONS "un."

; Lets shells started from now on through Explorer see the changed PATH.
!macro DOWNLIGHT_ANNOUNCE_ENVIRONMENT
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:${DOWNLIGHT_ENVIRONMENT}" /TIMEOUT=5000
!macroend

Function DownlightAddToPath
  Push $R0 ; how the PATH read, then the length of the PATH to write
  Push $R1 ; the PATH's last character, then the length of the install folder
  Push $R2 ; the separator before the install folder
  Call DownlightReadPath
  Pop $R0
  ${If} $R0 == "unsafe"
    MessageBox MB_ICONINFORMATION|MB_OK "Downlight couldn't add itself to your PATH, because your PATH is longer than the installer can change safely. To start Downlight from a shell, add this folder to your PATH yourself:$\r$\n$\r$\n$INSTDIR" /SD IDOK
    DetailPrint "Not added to the PATH: it is too long to change safely."
  ${Else}
    Call DownlightLeaveOutInstallFolder
    ${If} $DownlightPathHadIt = 1
      DetailPrint "The PATH already holds $INSTDIR."
    ${Else}
      StrCpy $R1 $DownlightPath 1 -1
      ${If} $DownlightPath == ""
      ${OrIf} $R1 == ";"
        StrCpy $R2 ""
      ${Else}
        StrCpy $R2 ";"
      ${EndIf}
      ; NSIS cuts a string longer than NSIS_MAX_STRLEN short without saying so, so measure the
      ; PATH to write, and its terminating null, before putting it together.
      StrLen $R0 "$DownlightPath$R2"
      StrLen $R1 $INSTDIR
      IntOp $R0 $R0 + $R1
      IntOp $R0 $R0 + 1
      ${If} $R0 > ${NSIS_MAX_STRLEN}
        MessageBox MB_ICONINFORMATION|MB_OK "Downlight couldn't add itself to your PATH, because your PATH would grow longer than the installer can write safely. To start Downlight from a shell, add this folder to your PATH yourself:$\r$\n$\r$\n$INSTDIR" /SD IDOK
        DetailPrint "Not added to the PATH: it would grow too long to write safely."
      ${Else}
        ; The user's PATH is REG_EXPAND_SZ, as Windows writes it, so that entries like
        ; %USERPROFILE%\bin keep working.
        WriteRegExpandStr HKCU "${DOWNLIGHT_ENVIRONMENT}" "Path" "$DownlightPath$R2$INSTDIR"
        !insertmacro DOWNLIGHT_ANNOUNCE_ENVIRONMENT
        DetailPrint "Added $INSTDIR to the PATH."
      ${EndIf}
    ${EndIf}
  ${EndIf}
  Pop $R2
  Pop $R1
  Pop $R0
FunctionEnd

Function un.DownlightRemoveFromPath
  Push $R0
  Call un.DownlightReadPath
  Pop $R0
  ${If} $R0 == "unsafe"
    MessageBox MB_ICONINFORMATION|MB_OK "Your PATH is longer than the uninstaller can change safely, so if it holds this folder, remove it yourself:$\r$\n$\r$\n$INSTDIR" /SD IDOK
    DetailPrint "Not removed from the PATH: it is too long to change safely."
  ${ElseIf} $R0 == "ok"
    Call un.DownlightLeaveOutInstallFolder
    ${If} $DownlightPathHadIt = 1
      ${If} $DownlightPathWithout == ""
        DeleteRegValue HKCU "${DOWNLIGHT_ENVIRONMENT}" "Path"
      ${Else}
        WriteRegExpandStr HKCU "${DOWNLIGHT_ENVIRONMENT}" "Path" $DownlightPathWithout
      ${EndIf}
      !insertmacro DOWNLIGHT_ANNOUNCE_ENVIRONMENT
      DetailPrint "Removed $INSTDIR from the PATH."
    ${EndIf}
  ${EndIf}
  Pop $R0
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
  ; The ProgID that each extension's OpenWithProgids and the Capabilities name.
  WriteRegStr SHCTX "Software\Classes\${DOWNLIGHT_PROGID}" "" "Markdown document"
  WriteRegStr SHCTX "Software\Classes\${DOWNLIGHT_PROGID}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHCTX "Software\Classes\${DOWNLIGHT_PROGID}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""

  WriteRegStr SHCTX "${MANUPRODUCTKEY}\Capabilities" "ApplicationName" "${PRODUCTNAME}"
  WriteRegStr SHCTX "${MANUPRODUCTKEY}\Capabilities" "ApplicationDescription" "A small, fast viewer for Markdown files."
  !insertmacro DOWNLIGHT_FOR_EACH_EXTENSION DOWNLIGHT_ASSOCIATE
  WriteRegStr SHCTX "Software\RegisteredApplications" "${PRODUCTNAME}" "${MANUPRODUCTKEY}\Capabilities"
  !insertmacro UPDATEFILEASSOC

  Call DownlightAddToPath
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; UpdateMode is 1 when the uninstaller runs with /UPDATE, while a newer version installs over
  ; this one. The associations and the PATH entry stay then, as Tauri's own uninstall keeps the
  ; shortcuts, so the reader's default app and shells never lose Downlight in between.
  ${If} $UpdateMode <> 1
    DeleteRegValue SHCTX "Software\RegisteredApplications" "${PRODUCTNAME}"
    !insertmacro DOWNLIGHT_FOR_EACH_EXTENSION DOWNLIGHT_UNASSOCIATE
    DeleteRegKey SHCTX "${MANUPRODUCTKEY}\Capabilities"
    DeleteRegKey SHCTX "Software\Classes\${DOWNLIGHT_PROGID}"
    !insertmacro UPDATEFILEASSOC

    Call un.DownlightRemoveFromPath
  ${EndIf}
!macroend
