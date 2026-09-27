#!/usr/bin/env python3

import argparse
import struct

from pathlib import Path
from bisect import bisect_right

from elftools.elf.elffile import ELFFile
from elftools.dwarf.descriptions import describe_form_class


FILE_NAME_SIZE = 64
FUNCTION_NAME_SIZE = 64

ENTRY_FORMAT = (
    f"<QI"
    f"{FILE_NAME_SIZE}s"
    f"{FUNCTION_NAME_SIZE}s"
    f"4x"
)

ENTRY_SIZE = struct.calcsize(ENTRY_FORMAT)


def encode_fixed_string(
    value: str,
    size: int,
) -> bytes:
    encoded = value.encode(
        "utf-8",
        errors="replace",
    )

    # reserve one byte for the null terminator
    encoded = encoded[:size - 1]

    return encoded.ljust(size, b"\0")


def decode_dwarf_string(value) -> str:
    if isinstance(value, bytes):
        return value.decode(
            "utf-8",
            errors="replace",
        )

    return str(value)


def get_file_name(
    line_program,
    file_index: int,
) -> str:
    if file_index == 0:
        return "unknown"

    file_entries = line_program["file_entry"]
    index = file_index - 1

    if index >= len(file_entries):
        return "unknown"

    file_entry = file_entries[index]

    try:
        return decode_dwarf_string(
            file_entry.name
        )
    except Exception:
        return "unknown"


def get_die_name(die) -> str:
    attributes = die.attributes

    # prefer the normal source-level function name
    name = attributes.get("DW_AT_name")

    if name is not None:
        return decode_dwarf_string(
            name.value
        )

    # Rust/C++ may expose a linkage name instead
    linkage_name = attributes.get(
        "DW_AT_linkage_name"
    )

    if linkage_name is None:
        linkage_name = attributes.get(
            "DW_AT_MIPS_linkage_name"
        )

    if linkage_name is not None:
        return decode_dwarf_string(
            linkage_name.value
        )

    return "unknown"

def get_die_range(die):
    attributes = die.attributes

    low_pc_attr = attributes.get("DW_AT_low_pc")
    high_pc_attr = attributes.get("DW_AT_high_pc")

    if low_pc_attr is None or high_pc_attr is None:
        return None

    low_pc = low_pc_attr.value

    high_pc_class = describe_form_class(
        high_pc_attr.form
    )

    if high_pc_class == "address":
        high_pc = high_pc_attr.value
    elif high_pc_class == "constant":
        high_pc = low_pc + high_pc_attr.value
    else:
        return None

    if high_pc <= low_pc:
        return None

    return low_pc, high_pc

def collect_functions(dwarf):
    functions = []

    for compile_unit in dwarf.iter_CUs():
        for die in compile_unit.iter_DIEs():
            if die.tag != "DW_TAG_subprogram":
                continue

            address_range = get_die_range(die)

            if address_range is None:
                continue

            low_pc, high_pc = address_range
            function_name = get_die_name(die)

            functions.append((
                low_pc,
                high_pc,
                function_name,
            ))

    functions.sort(
        key=lambda function: function[0]
    )

    return functions

def find_function(
    address: int,
    functions,
    function_starts,
) -> str:
    index = bisect_right(
        function_starts,
        address,
    ) - 1

    if index < 0:
        return "unknown"

    low_pc, high_pc, name = functions[index]

    if low_pc <= address < high_pc:
        return name

    return "unknown"


def generate_ndm(
    kernel_path: Path,
    output_path: Path,
) -> None:
    print(
        f"[NDM] Reading kernel ELF: "
        f"{kernel_path}"
    )

    entries = []

    with kernel_path.open("rb") as file:
        elf = ELFFile(file)

        if not elf.has_dwarf_info():
            raise RuntimeError(
                f"{kernel_path} contains no "
                f"DWARF information"
            )

        dwarf = elf.get_dwarf_info()

        functions = collect_functions(dwarf)

        function_starts = [
            function[0]
            for function in functions
        ]

        print(
            f"[NDM] Found "
            f"{len(functions)} functions"
        )

        for compile_unit in dwarf.iter_CUs():
            line_program = (
                dwarf.line_program_for_CU(
                    compile_unit
                )
            )

            if line_program is None:
                continue

            for line_entry in (
                line_program.get_entries()
            ):
                state = line_entry.state

                if (
                    state is None
                    or state.end_sequence
                    or not state.is_stmt
                ):
                    continue

                address = state.address
                line = state.line or 0

                file_name = get_file_name(
                    line_program,
                    state.file,
                )

                function_name = find_function(
                    address,
                    functions,
                    function_starts,
                )

                entries.append((
                    address,
                    line,
                    file_name,
                    function_name,
                ))

    entries.sort(
        key=lambda entry: entry[0]
    )

    output_path.parent.mkdir(
        parents=True,
        exist_ok=True,
    )

    with output_path.open("wb") as file:
        for (
            address,
            line,
            file_name,
            function_name,
        ) in entries:
            file_buffer = encode_fixed_string(
                file_name,
                FILE_NAME_SIZE,
            )

            function_buffer = (
                encode_fixed_string(
                    function_name,
                    FUNCTION_NAME_SIZE,
                )
            )

            file.write(
                struct.pack(
                    ENTRY_FORMAT,
                    address,
                    line,
                    file_buffer,
                    function_buffer,
                )
            )

    print(
        f"[NDM] Generated {len(entries)} entries"
    )

    print(
        f"[NDM] Entry size: "
        f"{ENTRY_SIZE} bytes"
    )

    print(
        f"[NDM] Output: {output_path}"
    )

    print(
        f"[NDM] Total size: "
        f"{len(entries) * ENTRY_SIZE} bytes"
    )

def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Generate a Nullex Debug Map "
            "from kernel DWARF information"
        )
    )

    parser.add_argument(
        "kernel",
        type=Path,
        help="Path to the linked kernel ELF",
    )

    parser.add_argument(
        "output",
        type=Path,
        help="Output NDM file",
    )

    args = parser.parse_args()

    generate_ndm(
        args.kernel,
        args.output,
    )

if __name__ == "__main__":
    main()