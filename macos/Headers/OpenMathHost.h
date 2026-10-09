#ifndef OPENMATH_HOST_H
#define OPENMATH_HOST_H
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
uint32_t om_host_abi_version(void);
/* Immutable static string; process lifetime. Never free or mutate. */
const char *om_host_kernel_release_version(void);
/* Descriptor linking probe only; no document/renderer readiness implied. */
uint32_t om_host_metadata_version(void);
#ifdef __cplusplus
}
#endif
#endif
