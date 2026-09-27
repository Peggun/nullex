import argparse

from dataclasses import replace

import elf_loader
import formatter

from resolver import SymbolResolver
from dwarf_lookup import DwarfResolver
from rust_demangler import demangle


def safe_demangle(name):
    if not name:
        return name

    try:
        result = demangle(name)

        if result:
            return result
    except Exception:
        pass

    return name


def main():
    parser = argparse.ArgumentParser(
        description=(
            "Resolve addresses to symbol names "
            "and source locations in an ELF file."
        )
    )

    parser.add_argument(
        "elf_file",
        help="Path to the ELF file.",
    )

    parser.add_argument(
        "addresses",
        nargs="+",
        help=(
            "Addresses to resolve "
            "(e.g. 0x400123)."
        ),
    )

    parser.add_argument(
        "--line",
        "-l",
        help=(
            "Resolve addresses to source "
            "line numbers using DWARF."
        ),
        action="store_true",
    )

    parser.add_argument(
        "--full",
        "-f",
        help="Display full symbol information.",
        action="store_true",
    )

    parser.add_argument(
        "--demangle",
        "-d",
        help="Demangle Rust symbol names.",
        action="store_true",
    )

    args = parser.parse_args()

    try:
        addresses = [
            int(address, 16)
            for address in args.addresses
        ]
    except ValueError as error:
        parser.error(
            f"invalid hexadecimal address: {error}"
        )

    symbols = elf_loader.load_symbols(
        args.elf_file
    )

    symbol_resolver = SymbolResolver(symbols)

    dwarf_resolver = (
        DwarfResolver(args.elf_file)
        if args.line
        else None
    )

    formatter.print_header(
        args.elf_file,
        symbols,
    )

    if (
        args.line
        and not dwarf_resolver.available
    ):
        formatter.print_warning(
            "No DWARF information available. "
            "Source line resolution is disabled."
        )

    for address in addresses:
        if address == 0:
            formatter.print_result(
                addr=address,
                symbol=None,
            )
            continue

        symbol, offset = symbol_resolver.resolve(
            address
        )

        line_info = (
            dwarf_resolver.resolve(address)
            if dwarf_resolver is not None
            else None
        )

        if symbol is not None and args.demangle:
            symbol = replace(
                symbol,
                name=safe_demangle(symbol.name),
            )

        formatter.print_result(
            addr=address,
            symbol=symbol,
            offset=offset,
            line_info=line_info,
            full=args.full,
        )


if __name__ == "__main__":
    main()