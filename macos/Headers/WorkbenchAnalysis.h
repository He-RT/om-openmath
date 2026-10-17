/* Read-only workbench v2 extension; base ABI remains version 1. */
#ifndef OPENMATH_WORKBENCH_ANALYSIS_H
#define OPENMATH_WORKBENCH_ANALYSIS_H
#include "OpenMathHost.h"
#ifdef __cplusplus
extern "C" {
#endif
om_host_buffer om_host_analyze_source(om_host_handle *host, const unsigned char *json, size_t len);
om_host_buffer om_host_list_definitions(om_host_handle *host, const unsigned char *json, size_t len);
#ifdef __cplusplus
}
#endif
#endif
