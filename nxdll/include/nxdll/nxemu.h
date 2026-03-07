#pragma once

#include <stdint.h>
#include "nxdll.h"

#ifdef __cplusplus
extern "C" {
#endif

    typedef struct {
        const char *name;
        uint16_t ordinal;
        const void *addr;
    } C_PEExportedFunction;

    typedef struct {
        C_PEExportedFunction *functions;
        size_t num_functions;
    } C_EmulatedDLL;

    RegisteredDllHandle* nx_register_emulated_dll(
        const char* dll_name,
        const C_EmulatedDLL* dll
    );

#ifdef __cplusplus
}
#endif
