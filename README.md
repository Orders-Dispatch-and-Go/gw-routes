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

[Service API (Wiki)](https://ai.nsu.ru/projects/gruzowiki/wiki/%D0%98%D0%BD%D1%82%D0%B5%D1%80%D1%84%D0%B5%D0%B9%D1%81_%22%D0%A1%D0%B5%D1%80%D0%B2%D0%B8%D1%81_%D1%80%D0%B0%D0%B1%D0%BE%D1%82%D1%8B_%D1%81_%D0%B7%D0%B0%D1%8F%D0%B2%D0%BA%D0%B0%D0%BC%D0%B8_%D0%B8_%D0%BF%D0%BE%D0%B5%D0%B7%D0%B4%D0%BA%D0%B0%D0%BC%D0%B8%22_-_%22%D0%A1%D0%B5%D1%80%D0%B2%D0%B8%D1%81_%D1%80%D0%B0%D0%B1%D0%BE%D1%82%D1%8B_%D1%81_%D0%91%D0%94%22)
