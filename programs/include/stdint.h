// stdint.h
#ifndef _STDINT_H
#define _STDINT_H

typedef signed char int8_t;
typedef short int int16_t;
typedef int int32_t;
typedef long long int int64_t;

typedef unsigned char uint8_t;
typedef unsigned short int uint16_t;
typedef unsigned int uint32_t;
typedef unsigned long long int uint64_t;

// assuming 64-bit
typedef __UINTPTR_TYPE__ uintptr_t;

#endif