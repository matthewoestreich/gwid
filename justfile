# Load subcommands from .just/test.just
mod test '.just/test.just'
# Load subcommands from .just/benches.just
mod benches '.just/benches.just'

# Default recipe: list all available commands
default:
    @just --list
