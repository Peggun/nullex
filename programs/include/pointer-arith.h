// https://github.com/akx/so55822235/blob/master/libc-pointer-arith.h

#include "stdint.h"

#ifndef _POINTER_ARITH_H
#define _POINTER_ARITH_H 1

#define ALIGN_DOWN(base, size) ((base) & -((__typeof__(base))(size)))

#define PTR_ALIGN_DOWN(base, size)                                             \
    ((__typeof__(base))ALIGN_DOWN((uintptr_t)(base), (size)))

#endif