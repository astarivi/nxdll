#ifndef SUM_NUMBERS_DLL_H
#define SUM_NUMBERS_DLL_H

#include <windows.h>

#ifdef __cplusplus
extern "C" {
#endif

__declspec(dllimport) int sum_numbers(int a, int b);

#ifdef __cplusplus
}
#endif

#endif