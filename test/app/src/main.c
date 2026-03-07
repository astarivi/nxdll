#include <hal/debug.h>
#include <hal/video.h>
#include <windows.h>
#include <nxdk/mount.h>
#include <nxdk/path.h>
#include <nxdll/nxdll.h>

typedef int (*sum_numbers_fn)(int a, int b);

int main(void)
{
    XVideoSetMode(640, 480, 32, REFRESH_DEFAULT);
    debugPrint("Starting up\n");
    nx_loader_init();

    RegisteredDllHandle* dll_handle = register_dll("Q:\\test.dll");
    LoadedDllHandle* instance_handle = load_dll(dll_handle);
    const unsigned char* fn_ptr = get_func_by_name(instance_handle, "sum_numbers");
    sum_numbers_fn sum = (sum_numbers_fn)fn_ptr;

    int result = sum(1, 2);

    debugPrint("%d\n", result);

    while(1) {
        Sleep(2000);
    }

    return 0;
}
