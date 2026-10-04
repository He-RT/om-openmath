#ifndef OPENMATH_KERNEL_H
#define OPENMATH_KERNEL_H
#include <stdint.h>
#include <stddef.h>
typedef struct { const uint8_t *data; size_t len; } OmBuffer;
uint32_t om_ios_abi_version(void);
uint64_t om_ios_create(const uint8_t *data, size_t len);
OmBuffer om_ios_request(uint64_t handle, const uint8_t *data, size_t len);
OmBuffer om_ios_http_bytes(uint64_t handle, uint64_t correlation, const uint8_t *request, size_t request_len, uint16_t status, const uint8_t *data, size_t len);
void om_ios_interrupt(uint64_t handle);
void om_ios_cancel(uint64_t handle, const uint8_t *data, size_t len);
void om_ios_destroy(uint64_t handle);
void om_ios_buffer_free(OmBuffer buffer);
#endif
