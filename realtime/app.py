from asyncio import get_event_loop, ensure_future
from faust import App, Worker

app = App('realtime', broker='kafka://localhost:9094', value_serializer='raw')
greetings_topic = app.topic('greetings')

@app.agent(greetings_topic)
async def greet(greetings):
    async for greeting in greetings:
        print(greeting)

def start_worker(app):
    print("starting faust worker...")
    worker = Worker(app, loglevel="INFO")
    worker.execute_from_commandline()