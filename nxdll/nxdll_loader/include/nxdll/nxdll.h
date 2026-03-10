#pragma once

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

    typedef struct RegisteredDllHandle RegisteredDllHandle;
    typedef struct LoadedDllHandle LoadedDllHandle;

    void nx_loader_init(void);

    RegisteredDllHandle* nx_register_dll(const char* c_path);
    void nx_unregister_dll(RegisteredDllHandle* handle);

    LoadedDllHandle* nx_load_dll(RegisteredDllHandle* handle);
    void nx_unload_dll(LoadedDllHandle* handle);

    const unsigned char* nx_get_func_by_ordinal(LoadedDllHandle* handle, uint16_t ordinal);
    const unsigned char* nx_get_func_by_name(LoadedDllHandle* handle, const char* func_name);

#ifdef __cplusplus
}
#endif
