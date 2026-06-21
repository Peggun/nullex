PROGRAM ?= $(notdir $(CURDIR))
OUT ?= $(PROGRAM).elf
BUILD ?= build

ARCH ?=

ifeq ($(ARCH),x86_64)
CC := x86_64-linux-gnu-gcc

CPPFLAGS += -I../include -I../lib -Isrc -I.
CFLAGS ?= -m64 -march=x86-64 -O2 -pipe -ffreestanding -fno-builtin \
 -fno-stack-protector -fno-common -fno-pie

LDFLAGS ?= -nostdlib -static -no-pie -Wl,--entry=_start -lgcc

else ifeq ($(ARCH),aarch64)
CC := aarch64-linux-gnu-gcc

CPPFLAGS += -I../include -I../lib -Isrc -I.
CFLAGS ?= -O2 -pipe -ffreestanding -fno-builtin \
 -fno-stack-protector -fno-common -fno-pie \
 -march=armv8-a

LDFLAGS ?= -nostdlib -static -no-pie -Wl,--entry=_start -lgcc

else
 $(error Unsupported ARCH '$(ARCH)')
endif

SRCS := ../_start.c $(wildcard src/*.c) $(filter-out main.c,$(wildcard *.c)) $(wildcard../lib/*.c)

OBJS := $(patsubst ../%.c,$(BUILD)/%.o,$(filter ../%.c,$(SRCS))) \
 $(patsubst src/%.c,$(BUILD)/src/%.o,$(filter src/%.c,$(SRCS))) \
 $(patsubst %.c,$(BUILD)/%.o,$(filter-out ../%.c src/%.c,$(SRCS)))

DEPS:= $(OBJS:.o=.d)

all: $(OUT)

$(OUT): $(OBJS)
	@echo "Linking $(OUT)"
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ $(OBJS) $(LDFLAGS)

#../_start.c - build/_start.o
$(BUILD)/%.o:../%.c
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) -MMD -MP -c $< -o $@

# src/foo.c - build/src/foo.o
$(BUILD)/src/%.o: src/%.c
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) -MMD -MP -c $< -o $@

# main.c -> build/main.o
$(BUILD)/%.o: %.c
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) -MMD -MP -c $< -o $@

-include $(DEPS)

clean:
	@rm -rf $(BUILD) $(OUT)

.PHONY: all clean