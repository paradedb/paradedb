import os
import click
import yaml
import psycopg2

from cli.endpoints import SourceTypeEnum, SourceTypeConverter
from engine.source_postgres import PostgresCDC


@click.group()
def cli():
    pass


@cli.command()
def init():
    # Create config.yaml file
    # Challenge is figuring out an extensible config.yaml format that can handle
    # multiple sources, sinks, and mappings between sources and sinks
    # I haven't figured out the best way to do this yet
    pass


@cli.command()
def watch():
    # Read in config.yaml file
    with open("config.yml", "r") as file:
        config = yaml.safe_load(file)

    dbname = config["dbname"]
    user = config["user"]
    host = config["host"]
    password = config["password"]
    table = config["table"]
    slot_name = config["slot_name"]
    port = config["port"]
    output_plugin = config["output_plugin"]
    publication_name = config["publication_name"]

    # Run Python listener
    try:
        print("Listening for changes...")
        cdc = PostgresCDC(
            publication_name=publication_name,
            slot_name=slot_name,
            database=dbname,
            host=host,
            user=user,
            password=password,
            port=port,
        )

        for event in cdc:
            print(event)

    except KeyboardInterrupt:
        print("Tearing down...")
        cdc.teardown()
        print("Teardown successful.")

    pass


if __name__ == "__main__":
    cli()
