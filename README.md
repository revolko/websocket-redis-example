# websocket-redis-example
Example of websocket server with Redis as connection storage.

## Note
This is not production ready service. The error handling is not done properly (I was being lazy) and 
redis worker does not clean the redis memory (so you need to delete the volume when you want to clean it; 
I hope to fix it in the future).

On the other side, feel free to contribute to this repository. I will accept all reasonable fixes.

## How to run
I have created a docker compose file that will build and run the service. Run:
```bash
docker compose up -d
```

Docker compose will start the webservice together with redis server.

### WS endpoint
The webservice exposes an endpoint for initiating WS connection:
`ws://127.0.0.1/ws/<client-id>` (where `<client-id>` must be unique for all clients). 
Actually, I did not handle the check of uniqueness of the id (will do in future), so I have 
no idea how it will behave if two different clients specify the same id.

### POST /messages/<client-id>
Another endpoint is for sending messages to the client: `http://127.0.0.1/messages/<client-id>`. 
As the webservice should serve as WS proxy, this endpoint can be used to send messages to specified 
client/websocket connection.
