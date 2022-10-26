use actix::{Actor, StreamHandler};
use actix_files::Files;
use actix_web::{
    main,
    web::{self, Payload},
    App, HttpServer,
};
use actix_web_actors::ws::{self, Message, ProtocolError, WebsocketContext};

#[main]
async fn main() -> Result<(), std::io::Error> {
    HttpServer::new(|| {
        App::new()
            .route(
                "/websocket",
                web::get().to(|req, stream: Payload| async move {
                    ws::start(WsHandler {}, &req, stream)
                }),
            )
            .service(Files::new("/", "./static/").index_file("index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .bind(("[::]", 8080))?
    .run()
    .await
}

struct WsHandler;

impl Actor for WsHandler {
    type Context = WebsocketContext<Self>;
}

impl StreamHandler<Result<Message, ProtocolError>> for WsHandler {
    fn handle(&mut self, item: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        todo!()
    }
}
