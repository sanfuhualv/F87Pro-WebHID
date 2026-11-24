Unicode true
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 32
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "nsDialogs.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

!ifndef APP_EXE
  !error "APP_EXE must point to the built desktop executable"
!endif
!ifndef APP_VERSION
  !error "APP_VERSION is required"
!endif
!ifndef OUTPUT_FILE
  !error "OUTPUT_FILE is required"
!endif
!ifndef WEBVIEW2_BOOTSTRAPPER
  !error "WEBVIEW2_BOOTSTRAPPER is required"
!endif
!define APP_NAME "F87 Pro Desktop"
!define APP_KEY "Software\F87ProDesktop"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\F87ProDesktop"
!define RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define EDGE_CLIENTS "Software\Microsoft\EdgeUpdate\Clients"

Name "${APP_NAME}"
OutFile "${OUTPUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "${APP_KEY}" "InstallDir"
Icon "..\src-tauri\icons\icon.ico"
UninstallIcon "..\src-tauri\icons\icon.ico"
VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey /LANG=2052 "ProductName" "${APP_NAME} 安装程序"
VIAddVersionKey /LANG=2052 "FileDescription" "${APP_NAME} 安装程序"
VIAddVersionKey /LANG=2052 "ProductVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=2052 "FileVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=2052 "LegalCopyright" "F87 Pro Desktop contributors"
BrandingText "${APP_NAME}"
ShowInstDetails show
ShowUninstDetails show

Var OptionsDialog
Var DesktopCheckbox
Var AutostartCheckbox
Var DesktopChoice
Var AutostartChoice
Var RuntimeVersion

!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "欢迎安装 ${APP_NAME}"
!define MUI_WELCOMEPAGE_TEXT "安装后可以使用完整键盘驱动和序列帧后台播放。$\r$\n$\r$\n您可以选择安装目录、桌面快捷方式和开机自启动。自启动时只运行后台托盘，不弹出界面。$\r$\n$\r$\n若系统缺少 WebView2，安装时会联网安装微软运行库。"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
Page custom OptionsPage OptionsLeave
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_NAME}.exe"
!define MUI_FINISHPAGE_RUN_TEXT "立即打开 ${APP_NAME}"
!define MUI_FINISHPAGE_TEXT "${APP_NAME} 已安装完成。$\r$\n$\r$\n正常打开会显示驱动界面；开机自启动仅显示托盘图标。关闭窗口后程序仍在后台运行，可从托盘退出。"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH
!insertmacro MUI_LANGUAGE "SimpChinese"

Function .onInit
  SetShellVarContext current
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "此版本需要 64 位 Windows。" /SD IDOK
    SetErrorLevel 1
    Quit
  ${EndIf}
  SetRegView 64
  StrCpy $DesktopChoice ${BST_CHECKED}
  StrCpy $AutostartChoice ${BST_UNCHECKED}
  ClearErrors
  ReadRegDWORD $0 HKCU "${APP_KEY}" "DesktopShortcut"
  ${IfNot} ${Errors}
    StrCpy $DesktopChoice $0
  ${EndIf}
  ClearErrors
  ReadRegDWORD $0 HKCU "${APP_KEY}" "Autostart"
  ${IfNot} ${Errors}
    StrCpy $AutostartChoice $0
  ${EndIf}
FunctionEnd

Function OptionsPage
  !insertmacro MUI_HEADER_TEXT "安装选项" "选择快捷方式与后台自启动方式"
  nsDialogs::Create 1018
  Pop $OptionsDialog
  ${If} $OptionsDialog == error
    Abort
  ${EndIf}
  ${NSD_CreateCheckbox} 0 10u 100% 16u "在桌面创建 F87 Pro Desktop 快捷方式"
  Pop $DesktopCheckbox
  ${NSD_SetState} $DesktopCheckbox $DesktopChoice
  ${NSD_CreateCheckbox} 0 40u 100% 16u "开机自启动（登录 Windows 后静默运行，不弹出界面）"
  Pop $AutostartCheckbox
  ${NSD_SetState} $AutostartCheckbox $AutostartChoice
  ${NSD_CreateLabel} 0 76u 100% 54u "自启动只启动后台驱动与托盘，不会自动播放动画。$\r$\n需要界面时，双击桌面快捷方式或点击托盘的“打开F87ProDesktop”。$\r$\n动画、草稿和登录数据保存在用户目录，卸载时保留。"
  Pop $0
  nsDialogs::Show
FunctionEnd

Function OptionsLeave
  ${NSD_GetState} $DesktopCheckbox $DesktopChoice
  ${NSD_GetState} $AutostartCheckbox $AutostartChoice
FunctionEnd

Function CheckAppRunning
  System::Call 'kernel32::OpenMutexW(i 0x100000, i 0, w "Local\F87ProDesktopLighting-0.1") p.r0'
  ${If} $0 != 0
    System::Call 'kernel32::CloseHandle(p r0)'
    MessageBox MB_OK|MB_ICONEXCLAMATION "F87 Pro Desktop 正在运行。请先从托盘选择“退出”，然后重新安装或卸载。" /SD IDOK
    SetErrorLevel 2
    Abort
  ${EndIf}
FunctionEnd

!macro ProbeRuntime ROOT TAG
  ; Microsoft Evergreen WebView2 Runtime has a fixed EdgeUpdate client ID.
  ; Never enumerate until an error: EnumRegKey can return an empty name at EOF.
  ClearErrors
  ReadRegStr $4 ${ROOT} "${EDGE_CLIENTS}\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  IfErrors probe_done_${TAG}
  StrCmp $4 "" probe_done_${TAG}
  StrCmp $4 "0.0.0.0" probe_done_${TAG}
  StrCpy $RuntimeVersion $4
  Goto runtime_found
probe_done_${TAG}:
!macroend

Function FindWebView2
  StrCpy $RuntimeVersion ""
  SetRegView 32
  !insertmacro ProbeRuntime HKCU user32
  !insertmacro ProbeRuntime HKLM machine32
  SetRegView 64
  !insertmacro ProbeRuntime HKCU user64
  !insertmacro ProbeRuntime HKLM machine64
runtime_found:
  SetRegView 64
FunctionEnd

Function EnsureWebView2
  Call FindWebView2
  ${If} $RuntimeVersion != ""
    DetailPrint "WebView2 已安装：$RuntimeVersion"
    Return
  ${EndIf}
  DetailPrint "正在安装 Microsoft Edge WebView2 Runtime，需要网络连接…"
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File /oname=MicrosoftEdgeWebview2Setup.exe "${WEBVIEW2_BOOTSTRAPPER}"
  nsExec::ExecToLog /TIMEOUT=120000 '"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe" /silent /install'
  Pop $0
  DetailPrint "WebView2 引导程序结果：$0"
  Call FindWebView2
  ${If} $RuntimeVersion == ""
    MessageBox MB_OK|MB_ICONSTOP "WebView2 未能安装。请检查网络，安装微软 WebView2 Runtime 后重新运行本安装程序。" /SD IDOK
    SetErrorLevel 3
    Abort
  ${EndIf}
FunctionEnd

Section "安装" MainSection
  DetailPrint "正在检查程序运行状态…"
  Call CheckAppRunning
  DetailPrint "正在检查 WebView2 Runtime…"
  Call EnsureWebView2
  DetailPrint "正在安装程序文件…"
  SetShellVarContext current
  SetRegView 64
  SetOutPath "$INSTDIR"
  File "/oname=F87 Pro Desktop.exe" "${APP_EXE}"
  File /oname=LICENSE "..\..\LICENSE"
  File /oname=THIRD_PARTY_NOTICES.md "..\..\THIRD_PARTY_NOTICES.md"
  SetOutPath "$INSTDIR\third-party-licenses"
  File /r "..\licenses\generated\*"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\${APP_NAME}.exe"
  ${If} $DesktopChoice == ${BST_CHECKED}
    CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_NAME}.exe"
  ${Else}
    Delete "$DESKTOP\${APP_NAME}.lnk"
  ${EndIf}
  ${If} $AutostartChoice == ${BST_CHECKED}
    WriteRegStr HKCU "${RUN_KEY}" "F87ProDesktop" '"$INSTDIR\${APP_NAME}.exe" --autostart'
  ${Else}
    DeleteRegValue HKCU "${RUN_KEY}" "F87ProDesktop"
  ${EndIf}
  WriteRegStr HKCU "${APP_KEY}" "InstallDir" "$INSTDIR"
  WriteRegDWORD HKCU "${APP_KEY}" "DesktopShortcut" $DesktopChoice
  WriteRegDWORD HKCU "${APP_KEY}" "Autostart" $AutostartChoice
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "F87 Pro Desktop"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" '"$INSTDIR\${APP_NAME}.exe",0'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" $0
SectionEnd

Function un.onInit
  SetShellVarContext current
  SetRegView 64
  System::Call 'kernel32::OpenMutexW(i 0x100000, i 0, w "Local\F87ProDesktopLighting-0.1") p.r0'
  ${If} $0 != 0
    System::Call 'kernel32::CloseHandle(p r0)'
    MessageBox MB_OK|MB_ICONEXCLAMATION "请先从托盘退出 F87 Pro Desktop，再进行卸载。" /SD IDOK
    SetErrorLevel 2
    Abort
  ${EndIf}
FunctionEnd

Section "Uninstall"
  ReadRegStr $0 HKCU "${APP_KEY}" "InstallDir"
  ${If} $0 == $INSTDIR
    Delete "$DESKTOP\${APP_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}.lnk"
    DeleteRegValue HKCU "${RUN_KEY}" "F87ProDesktop"
    DeleteRegKey HKCU "${UNINSTALL_KEY}"
    DeleteRegKey HKCU "${APP_KEY}"
  ${EndIf}
  Delete "$INSTDIR\${APP_NAME}.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  RMDir /r "$INSTDIR\third-party-licenses"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  ; User animation files and WebView data are deliberately outside the install folder.
SectionEnd
