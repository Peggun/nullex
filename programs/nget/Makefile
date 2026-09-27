
PROGRAM ?= $(notdir $(CURDIR))
OUT ?= $(PROGRAM).elf
BUILD ?= build

CC := x86_64-linux-gnu-gcc

NULLEX_INCLUDE := $(abspath ../include)
NULLEX_LIB := $(abspath ../lib)

CPPFLAGS += -nostdinc
CPPFLAGS += -I$(NULLEX_INCLUDE) -I$(NULLEX_LIB) -Isrc -I.

CFLAGS ?= -m64 -march=x86-64 -O2 -pipe -ffreestanding -fno-builtin \
 -fno-stack-protector -fno-common -fno-pie

LDFLAGS ?= -nostdlib -static -no-pie -Wl,--entry=_start -lgcc

SRCS := ../_start.c $(wildcard src/*.c) $(filter-out main.c,$(wildcard *.c)) $(wildcard ../lib/*.c)

OBJS := $(patsubst ../%.c,$(BUILD)/%.o,$(filter ../%.c,$(SRCS))) \
 $(patsubst src/%.c,$(BUILD)/src/%.o,$(filter src/%.c,$(SRCS))) \
 $(patsubst %.c,$(BUILD)/%.o,$(filter-out ../%.c src/%.c,$(SRCS)))

DEPS:= $(OBJS:.o=.d)

all: $(OUT)

$(OUT): $(OBJS)
	@echo "Linking $(OUT)"
	@mkdir -p $(@D)
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ $(OBJS) $(LDFLAGS)

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