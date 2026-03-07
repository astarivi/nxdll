#include <windows.h>

__declspec(dllexport) int sum_numbers(int a, int b) {
    return a + b;
}

BOOL APIENTRY DllMain(HMODULE hModule,
                      DWORD  ul_reason_for_call,
                      LPVOID lpReserved) {
    switch (ul_reason_for_call) {
    case 0:
    case 1:
    case 2:
    case 3:
        break;
    }
    return TRUE;
}
