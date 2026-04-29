from pathlib import Path


class Service:
    def run(self, name: str) -> str:
        return Path(name).read_text()


def process_request(value: str) -> str:
    """Process a request value."""
    return value.strip()
