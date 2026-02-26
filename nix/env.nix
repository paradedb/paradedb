{
  pkgs,
  pgVersion,
  postgresql,
  pg_search,
}:

pkgs.mkShellNoCC {
  packages = [
    (postgresql.withPackages (_: [
      pg_search
    ]))
  ];

  shellHook = ''
    export PGDATA="$PWD/.pg/v${toString pgVersion}"
    export PGHOST="$PGDATA"
    export PGDATABASE="postgres"

    start-pg() {
      if [ ! -d "$PGDATA" ]; then
        initdb --no-locale --encoding=UTF8 > /dev/null 2>&1
      fi

      pg_ctl start -l "$PGDATA/log" -o "-k $PGHOST -h '''"
      psql --command "CREATE EXTENSION IF NOT EXISTS pg_search;"
    }

    seed-pg() {
      psql --command "${builtins.readFile ./sql/setup.sql}"
    }

    stop-pg() {
      pg_ctl stop
    }

    echo "Development environment for pg_search on PostgreSQL ${toString pgVersion} 🚀"
    echo "Created PostgreSQL database \"$PGDATABASE\" and data directory at $PGDATA ✅"
    echo "Available commands:"
    echo "    start-pg    Start up PostgreSQL with the pg_search extension already created"
    echo "    seed-pg     Seed Postgres with the setup data from the pg_search quickstart tutorial"
    echo "    stop-pg     Stop the running Postgres instance"
  '';
}
