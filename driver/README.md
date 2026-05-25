# AgentGuard Kernel Minifilter — Phase 2
#
# C++ driver (WDK). Intercepta operaciones de fichero a nivel kernel.
# Altitude: 320000
#
# No compila en el workspace Rust. Se construye con MSBuild/Visual Studio + WDK.
#
# Carpetas esperadas cuando se implemente:
#   driver/
#     agentguard.sys         ← binario firmado
#     src/
#       minifilter.cpp       ← DriverEntry, FLT_PREOP_CALLBACK
#       process_notify.cpp   ← PsSetCreateProcessNotifyRoutineEx2
#       fltport.cpp          ← FltCommunicationPort
#       Makefile / sources   ← WDK build
#
# NO tocar hasta Phase 2.
