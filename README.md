# Routes Service

## Config

Environment:
- `PG_URL`: Postgres connection string
- `LISTEN_PORT`: Which port should the service listen on
- `MAP_SERVICE_ADDR`: Map Service URL. Should start with the proto (http://)
- `RUST_LOG`: Log Level (error, warn, info, debug, trace)

## Build

### Clone the repo

```bash
git clone --recurse-submodules https://github.com/Orders-Dispatch-and-Go/gw-routes.git && cd gw-routes
```

### Build using Docker

```bash
docker build .
```

## API

[Service API (Wiki)](https://github.com/Orders-Dispatch-and-Go/gruzowiki-transportation-go/blob/main/back-bd_api-v1.md)
