import os
import sqlite3
from pathlib import Path


def main():
    db_file = Path(os.environ["AGENT_SWARM_SQLITE_DB"])
    migration_file = Path(os.environ["AGENT_SWARM_SQLITE_MIGRATION"])

    schema = migration_file.read_text(encoding="utf-8")

    with sqlite3.connect(db_file) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        connection.executescript(schema)
        connection.commit()

    print(f"SQLite initialized: {db_file}")


if __name__ == "__main__":
    main()
