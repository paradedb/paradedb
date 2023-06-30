import click


# Available source types
# TODO: MySQL, MongoDB, etc.
class SourceTypeEnum:
    POSTGRES = "postgres"


# Available sink types
# TODO: Chroma, etc.
class SinkTypeEnum:
    PINECONE = "pinecone"


class SourceType(click.ParamType):
    name = "source_name"

    def convert(self, value, param, ctx):
        if value.upper() in (SourceTypeEnum.POSTGRES.upper(),):
            return value
        else:
            message = f'Invalid source type "{value}". Available options are: {", ".join([SourceTypeEnum.POSTGRES])}.'
            self.fail(message, param, ctx)


class SinkType(click.ParamType):
    name = "sink_name"

    def convert(self, value, param, ctx):
        if value.upper() in (SinkTypeEnum.POSTGRES.upper(),):
            return value
        else:
            message = f'Invalid sink type "{value}". Available options are: {", ".join([SinkTypeEnum.PINECONE])}.'
            self.fail(message, param, ctx)


SourceTypeConverter = SourceType()
SinkTypeConverter = SinkType()
