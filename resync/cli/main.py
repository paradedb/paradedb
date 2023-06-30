import os
import click
import yaml
import psycopg2

from endpoints import SourceTypeEnum, SourceTypeConverter


@click.group()
def cli():
    pass


@cli.group()
def source():
    pass


@cli.group()
def sink():
    pass


@cli.command()
def watch():
    # Run Python listener
    pass


@source.command()
def add(source_type, host, user, password, db_name, table_name):
    pass


@source.command()
def remove():
    pass


@source.command()
def list():
    pass


if __name__ == "__main__":
    cli()
