typedef void* HINSTANCE;
typedef unsigned long DWORD;
typedef void* LPVOID;
typedef int BOOL;

#define WINAPI __stdcall
#define TRUE 1

BOOL WINAPI __DllMainCRTStartup(
    HINSTANCE hinst,
    DWORD reason,
    LPVOID reserved)
{
    return TRUE;
}