from dataclasses import dataclass

from elftools.elf.elffile import ELFFile
from elftools.elf.sections import SymbolTableSection


@dataclass(slots=True, frozen=True)
class Symbol:
    address: int
    name: str
    size: int
    type: str
    binding: str
    visibility: str
    section: str


def load_symbols(elf_file_path):
    symbols = []

    with open(elf_file_path, "rb") as f:
        elf = ELFFile(f)

        for symbol_table in elf.iter_sections():
            if not isinstance(symbol_table, SymbolTableSection):
                continue

            for symbol in symbol_table.iter_symbols():
                name = symbol.name
                address = symbol["st_value"]
                symbol_type = symbol["st_info"]["type"]

                if not name or address == 0:
                    continue

                if symbol_type not in ("STT_FUNC", "STT_OBJECT"):
                    continue

                section_name = "UNKNOWN"
                section_index = symbol["st_shndx"]

                if isinstance(section_index, int):
                    target_section = elf.get_section(section_index)

                    if target_section is not None:
                        section_name = target_section.name

                symbols.append(
                    Symbol(
                        address=address,
                        name=name,
                        size=symbol["st_size"],
                        type=symbol_type,
                        binding=symbol["st_info"]["bind"],
                        visibility=symbol["st_other"]["visibility"],
                        section=section_name,
                    )
                )

    symbols.sort(key=lambda symbol: symbol.address)

    unique = []
    seen = set()

    for symbol in symbols:
        key = (symbol.address, symbol.name)

        if key in seen:
            continue

        seen.add(key)
        unique.append(symbol)

    return unique