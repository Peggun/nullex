
PROGRAM ?= $(notdir $(CURDIR))
OUT ?= $(PROGRAM).elf
BUILD ?= build

CC := x86_64-linux-gnu-gcc

NULLEX_INCLUDE := $(abspath ../include)
NULLEX_LIB := $(abspath ../lib)

LDSCRIPT ?= $(abspath ../link.ld)
ifeq ($(wildcard $(LDSCRIPT)),)
$(error Linker script not found: $(LDSCRIPT))
endif

CPPFLAGS += -nostdinc
CPPFLAGS += -I$(NULLEX_INCLUDE) -I$(NULLEX_LIB) -Isrc -I.

CFLAGS ?= -m64 -march=x86-64 -O2 -pipe -ffreestanding -fno-builtin \
 -fno-stack-protector -fno-common -fno-pie -fno-builtin-memcpy -fno-builtin-memset -fno-builtin-strrchr -fno-builtin-strncpy

LDFLAGS := -nostdlib -static -no-pie
LDFLAGS += -Wl,--entry=_start
LDFLAGS += -Wl,-T,$(LDSCRIPT)
LDFLAGS += -Wl,-z,max-page-size=0x1000 -Wl,-z,common-page-size=0x1000

LDLIBS := -lgcc

SRCS := ../_start.c $(wildcard src/*.c) $(filter-out main.c,$(wildcard *.c)) $(wildcard ../lib/*.c)

OBJS := $(patsubst ../%.c,$(BUILD)/%.o,$(filter ../%.c,$(SRCS))) \
 $(patsubst src/%.c,$(BUILD)/src/%.o,$(filter src/%.c,$(SRCS))) \
 $(patsubst %.c,$(BUILD)/%.o,$(filter-out ../%.c src/%.c,$(SRCS)))

DEPS:= $(OBJS:.o=.d)

all: $(OUT)

$(OUT): $(OBJS)
	@echo "Linking $(OUT)"
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) $(LDFLAGS) -o $@ $(OBJS) $(LDLIBS)

$(BUILD)/%.o: ../%.c
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) -MMD -MP -c $< -o $@

$(BUILD)/src/%.o: src/%.c
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) -MMD -MP -c $< -o $@

$(BUILD)/%.o: %.c
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) -MMD -MP -c $< -o $@

-include $(DEPS)

clean:
	@rm -rf $(BUILD) $(OUT)

.PHONY: all clean