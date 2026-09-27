import bisect


class SymbolResolver:
    __slots__ = ("symbols", "addresses")

    def __init__(self, symbols):
        self.symbols = symbols
        self.addresses = [
            symbol.address
            for symbol in symbols
        ]

    def resolve(self, address):
        if not self.symbols:
            return None, None

        index = bisect.bisect_right(
            self.addresses,
            address,
        ) - 1

        if index < 0:
            return None, None

        symbol = self.symbols[index]
        offset = address - symbol.address

        # ff the symbol has a known size, reject addresses
        # outside of it.
        if symbol.size > 0 and offset >= symbol.size:
            return None, None

        return symbol, offset