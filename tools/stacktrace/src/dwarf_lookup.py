import bisect

from pathlib import Path

from elftools.elf.elffile import ELFFile


class DwarfResolver:
    __slots__ = (
        "available",
        "entries",
        "addresses",
    )

    def __init__(self, file_path):
        self.available = False
        self.entries = []
        self.addresses = []

        self._load(file_path)

    def _load(self, file_path):
        with open(file_path, "rb") as f:
            elf = ELFFile(f)

            if not elf.has_dwarf_info():
                return

            self.available = True
            dwarf = elf.get_dwarf_info()

            for cu in dwarf.iter_CUs():
                line_program = dwarf.line_program_for_CU(cu)

                if line_program is None:
                    continue

                top_die = cu.get_top_DIE()

                comp_dir = ""
                comp_dir_attr = top_die.attributes.get(
                    "DW_AT_comp_dir"
                )

                if comp_dir_attr:
                    comp_dir = self._decode(
                        comp_dir_attr.value
                    )

                previous = None

                for entry in line_program.get_entries():
                    state = entry.state

                    if state is None:
                        continue

                    if state.end_sequence:
                        previous = None
                        continue

                    if (
                        previous is not None
                        and previous.address < state.address
                    ):
                        location = self._resolve_location(
                            line_program,
                            comp_dir,
                            previous,
                        )

                        if location is not None:
                            self.entries.append(
                                (
                                    previous.address,
                                    state.address,
                                    location,
                                )
                            )

                    previous = state

        self.entries.sort(
            key=lambda entry: entry[0]
        )

        self.addresses = [
            entry[0]
            for entry in self.entries
        ]

    def resolve(self, address):
        if not self.available or not self.entries:
            return None

        index = bisect.bisect_right(
            self.addresses,
            address,
        ) - 1

        if index < 0:
            return None

        start, end, location = self.entries[index]

        if not start <= address < end:
            return None

        return location

    def _resolve_location(
        self,
        line_program,
        comp_dir,
        state,
    ):
        if state.file == 0:
            return None

        file_entries = line_program["file_entry"]

        file_index = state.file - 1

        if file_index >= len(file_entries):
            return None

        file_entry = file_entries[file_index]

        filename = self._decode(file_entry.name)
        include_dir = ""

        if file_entry.dir_index != 0:
            directories = line_program[
                "include_directory"
            ]

            directory_index = (
                file_entry.dir_index - 1
            )

            if directory_index < len(directories):
                include_dir = self._decode(
                    directories[directory_index]
                )

        full_path = Path(comp_dir)

        if include_dir:
            full_path /= include_dir

        full_path /= filename

        try:
            full_path = (
                full_path.resolve()
                .relative_to(Path.cwd())
            )
        except (ValueError, OSError):
            try:
                full_path = full_path.resolve()
            except OSError:
                pass

        return str(full_path), state.line

    @staticmethod
    def _decode(value):
        if isinstance(value, bytes):
            return value.decode(
                "utf-8",
                errors="replace",
            )

        return str(value)


def is_dwarf_info_available(file_path):
    with open(file_path, "rb") as f:
        return ELFFile(f).has_dwarf_info()