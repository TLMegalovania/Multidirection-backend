use std::sync::Mutex;

use actix::{Actor, Addr, AsyncContext, Handler, StreamHandler};
use actix_files::Files;
use actix_web::{
    main,
    web::{self, Data, Payload},
    App, HttpRequest, HttpServer,
};
use actix_web_actors::ws::{
    self, CloseCode, CloseReason, Message, ProtocolError, WebsocketContext,
};
use bytestring::ByteString;

type WaitingData = Data<Mutex<Option<Addr<WsHandler>>>>;

#[main]
async fn main() -> Result<(), std::io::Error> {
    let global_data: WaitingData = WaitingData::new(Default::default());
    HttpServer::new(move || {
        App::new()
            .app_data(global_data.clone())
            .route(
                "/websocket",
                web::get().to(
                    |req: HttpRequest, stream: Payload, data: WaitingData| async move {
                        ws::start(
                            WsHandler {
                                enemy: None,
                                waiting: data,
                            },
                            &req,
                            stream,
                        )
                    },
                ),
            )
            .service(Files::new("/", "./static/").index_file("index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .bind(("[::]", 8080))?
    .run()
    .await
}

struct WsHandler {
    enemy: Option<Addr<WsHandler>>,
    waiting: WaitingData,
}

impl Actor for WsHandler {
    type Context = WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let mut waiting = self.waiting.lock().unwrap();
        if let Some(enemy) = waiting.take() {
            enemy.do_send(Mess::Enemy(ctx.address()));
            self.enemy = Some(enemy);
            ctx.text("black");
        } else {
            *waiting = Some(ctx.address());
        }
    }
    fn stopped(&mut self, ctx: &mut Self::Context) {
        let mut waiting = self.waiting.lock().unwrap();
        if matches!(*waiting, Some(ref enemy) if *enemy == ctx.address()) {
            *waiting = None;
        }
        if let Some(ref enemy) = self.enemy {
            enemy.do_send(Mess::Close);
        }
    }
}

impl StreamHandler<Result<Message, ProtocolError>> for WsHandler {
    fn handle(&mut self, item: Result<Message, ProtocolError>, _: &mut Self::Context) {
        let message = match item {
            Ok(it) => it,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
        if let (Message::Text(s), Some(ref enemy)) = (message, &self.enemy) {
            enemy.do_send(Mess::Go(s));
        }
    }
}

impl Handler<Mess> for WsHandler {
    type Result = ();

    fn handle(&mut self, msg: Mess, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            Mess::Enemy(enemy) => {
                if self.enemy.is_some() {
                    eprintln!("one receive a match while having matched");
                    return;
                }
                self.enemy = Some(enemy);
                ctx.text("white");
            }
            Mess::Close => ctx.close(Some(CloseReason {
                code: CloseCode::Normal,
                description: None,
            })),
            Mess::Go(s) => ctx.text(s),
        }
    }
}

enum Mess {
    Enemy(Addr<WsHandler>),
    Close,
    Go(ByteString),
}

impl actix::Message for Mess {
    type Result = ();
}
