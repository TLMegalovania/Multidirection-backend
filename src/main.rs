use std::{
    collections::HashMap,
    sync::{Mutex, RwLock},
};

use actix::{Actor, Addr, AsyncContext, Handler, StreamHandler};
use actix_files::Files;
use actix_web::{
    main,
    web::{self, Data, Payload},
    App, HttpRequest, HttpServer,
};
use actix_web_actors::ws::{self, Message, ProtocolError, WebsocketContext};

#[main]
async fn main() -> Result<(), std::io::Error> {
    let global_data = Data::new(WsMap::default());
    HttpServer::new(move || {
        App::new()
            .app_data(global_data.clone())
            .route(
                "/websocket",
                web::get().to(
                    |req: HttpRequest, stream: Payload, data: Data<WsMap>| async move {
                        ws::start(
                            WsHandler {
                                id: rand::random(),
                                map: data,
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
    id: u128,
    map: Data<WsMap>,
}

impl Actor for WsHandler {
    type Context = WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let mut waiting = self.map.waiting.lock().unwrap();
        if let Some(enemy_id) = *waiting {
            *waiting = None;
            // drop(waiting);
            let mut map = self.map.global_map.write().unwrap();
            let enemy = match map.remove(&enemy_id) {
                Some(e) => e,
                None => return, // should this panic?
            };
            map.insert(self.id, enemy.clone());
            map.insert(enemy_id, ctx.address());
            // drop(map);
            enemy.do_send(Mess::White);
            ctx.text("black");
        } else {
            *waiting = Some(self.id);
            // drop(waiting);
            self.map
                .global_map
                .write()
                .unwrap()
                .insert(self.id, ctx.address());
        }
    }
    fn stopped(&mut self, _: &mut Self::Context) {
        let mut waiting = self.map.waiting.lock().unwrap();

        if matches!(*waiting, Some(id) if id == self.id) {
            *waiting = None;
        }
        drop(waiting);

        let mut map = self.map.global_map.write().unwrap();
        let me_or_other = match map.remove(&self.id) {
            Some(h) => h,
            None => return,
        };
        drop(map);
        me_or_other.do_send(Mess::Close);
    }
}

impl StreamHandler<Result<Message, ProtocolError>> for WsHandler {
    fn handle(&mut self, item: Result<Message, ProtocolError>, _: &mut Self::Context) {
        let message = match item {
            Ok(it) => it,
            Err(_) => return,
        };
        if let Message::Text(s) = message {
            self.map.global_map.read().unwrap()[&self.id].do_send(Mess::Go(s.to_string()));
        }
    }
}

impl Handler<Mess> for WsHandler {
    type Result = ();

    fn handle(&mut self, msg: Mess, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            Mess::White => ctx.text("white"),
            Mess::Close => ctx.close(None),
            Mess::Go(s) => ctx.text(s),
        }
    }
}

#[derive(Default)]
struct WsMap {
    global_map: RwLock<HashMap<u128, Addr<WsHandler>>>,
    waiting: Mutex<Option<u128>>,
}

#[derive(actix::Message)]
#[rtype(result = "()")]
enum Mess {
    //Black,
    White,
    Close,
    Go(String),
}
