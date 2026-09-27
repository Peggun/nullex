import os

# formatting settings

INDENT = "    "

RESET = "\033[0m"

BOLD = "\033[1m"
DIM = "\033[2m"

RED = "\033[31m"
GREEN = "\033[32m"
YELLOW = "\033[33m"
BLUE = "\033[34m"
MAGENTA = "\033[35m"
CYAN = "\033[36m"
WHITE = "\033[37m"

BRIGHT_RED = "\033[91m"
BRIGHT_GREEN = "\033[92m"
BRIGHT_YELLOW = "\033[93m"
BRIGHT_BLUE = "\033[94m"
BRIGHT_MAGENTA = "\033[95m"
BRIGHT_CYAN = "\033[96m"


def print_header(elf_file, symbols):
    """Print the resolver header."""

    print(f"{BRIGHT_BLUE}{'=' * 70}{RESET}")
    print(f"{BOLD}{BRIGHT_CYAN}stacktrace.py for Nullex{RESET}")
    print(f"{BRIGHT_BLUE}{'=' * 70}{RESET}")

    print(f"{BOLD}ELF File{RESET} : {GREEN}{elf_file}{RESET}")
    print(f"{BOLD}Symbols {RESET} : {CYAN}{len(symbols)}{RESET}")
    print()


def print_warning(message):
    print(f"{BRIGHT_YELLOW}{BOLD}Warning:{RESET} {message}")
    print()


def print_error(message):
    print(f"{BRIGHT_RED}{BOLD}Error:{RESET} {message}")


def print_result(
    addr,
    symbol,
    offset=None,
    line_info=None,
    full=False,
):
    """Pretty-print a resolved address."""

    # address
    print(f"{BOLD}{CYAN}{addr:#018x}{RESET}")

    if symbol is None:
        print(f"{INDENT}{BRIGHT_RED}Unknown{RESET}")
        print()
        return

    # function + offset
    if offset is None:
        print(f"{INDENT}{BRIGHT_GREEN}{symbol.name}{RESET}")
    else:
        print(
            f"{INDENT}"
            f"{BRIGHT_GREEN}{symbol.name}{RESET}"
            f"{DIM}+0x{offset:x}{RESET}"
        )

    # source file
    if line_info:
        filename, line = line_info
        filename = os.path.normpath(filename)

        print(
            f"{INDENT}"
            f"{YELLOW}{filename}{RESET}"
            f"{DIM}:{line}{RESET}"
        )

    if full:
        print()

        info = [
            ("Address", f"{addr:#018x}"),
            ("Symbol", symbol.name),
            ("Symbol Addr", f"{symbol.address:#018x}"),
            ("Offset", f"0x{offset:x}" if offset is not None else "-"),
            ("Size", f"0x{symbol.size:x} ({symbol.size} bytes)"),
            ("Section", symbol.section),
            ("Type", symbol.type),
            ("Binding", symbol.binding),
            ("Visibility", symbol.visibility),
        ]

        if line_info:
            filename, line = line_info
            filename = os.path.normpath(filename)

            info.extend([
                ("File", filename),
                ("Line", str(line)),
            ])

        for key, value in info:
            print(
                f"{INDENT}"
                f"{BOLD}{BLUE}{key:<12}{RESET} "
                f"{WHITE}{value}{RESET}"
            )

    print()