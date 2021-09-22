use super::{
    message::{ClientMessage, Connect, WebsocketMessage},
    pc_usage::PcUsageSystem,
    python_repo::PythonRepoSystem,
};
use crate::{configuration::WebsocketSettings, websocket::message::WebsocketSystems};
use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Handler,
    StreamHandler, WrapFuture,
};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::time::Instant;
use uuid::Uuid;

#[tracing::instrument(
    name = "Starting web socket",
    skip(req, stream, websocket_settings, python_repo_system, pc_usage_system)
)]
pub async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    websocket_settings: web::Data<WebsocketSettings>,
    python_repo_system: web::Data<Addr<PythonRepoSystem>>,
    pc_usage_system: web::Data<Addr<PcUsageSystem>>,
) -> Result<HttpResponse, actix_web::Error> {
    let resp = ws::start(
        WebsocketSystem::new(
            websocket_settings.as_ref(),
            python_repo_system.get_ref().clone(),
            pc_usage_system.get_ref().clone(),
        ),
        &req,
        stream,
    );
    resp
}

struct WebsocketSystem {
    id: Uuid,
    hb: Instant,
    settings: WebsocketSettings,
    python_repo_system: Addr<PythonRepoSystem>,
    pc_usage_system: Addr<PcUsageSystem>,
}

impl WebsocketSystem {
    fn new(
        settings: &WebsocketSettings,
        python_repo_system: Addr<PythonRepoSystem>,
        pc_usage_system: Addr<PcUsageSystem>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            hb: Instant::now(),
            settings: settings.clone(),
            python_repo_system,
            pc_usage_system,
        }
    }

    /// Sends ping to client every x seconds.
    /// Also checks heathbeats from client.
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(self.settings.heartbeat_interval, |act, ctx| {
            // Check client heartbeats
            if Instant::now().duration_since(act.hb) > act.settings.client_timeout {
                // heartbeat timed out
                tracing::info!("Websocket client heartbeat failed, disconnecting.");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    #[tracing::instrument(name = "Process message", skip(self, ctx))]
    fn process_message(&self, text: &str, ctx: &mut ws::WebsocketContext<WebsocketSystem>) {
        match WebsocketMessage::parse(self.id, text) {
            Ok(message) => match message.system {
                WebsocketSystems::PythonRepo => self.python_repo_system.do_send(message.task),
                WebsocketSystems::PcUsage => self.pc_usage_system.do_send(message.task),
            },
            Err(e) => {
                tracing::error!("{:?}", e);
                ctx.address().do_send(e.into());
            }
        }
    }
}

impl Actor for WebsocketSystem {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        // Register to PythonRepoSystem
        self.python_repo_system
            .send(Connect {
                id: self.id,
                addr: ctx.address().recipient(),
            })
            .into_actor(self)
            .then(|res, _act, ctx| {
                if let Err(e) = res {
                    tracing::error!("Failed to connect to PythonRepoSystem: {:?}", e);
                    ctx.stop();
                }
                actix::fut::ready(())
            })
            .wait(ctx);

        // Register to PythonRepoSystem
        self.pc_usage_system
            .send(Connect {
                id: self.id,
                addr: ctx.address().recipient(),
            })
            .into_actor(self)
            .then(|res, _act, ctx| {
                if let Err(e) = res {
                    tracing::error!("Failed to connect to PcUsageSystem: {:?}", e);
                    ctx.stop();
                }
                actix::fut::ready(())
            })
            .wait(ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketSystem {
    #[tracing::instrument(
        name = "Handling websocket message",
        skip(self, item, ctx),
        fields(message=tracing::field::Empty)
    )]
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match item {
            Ok(msg) => msg,
            Err(e) => {
                tracing::error!("Unexpected error: {:?}", e);
                ctx.stop();
                return;
            }
        };
        tracing::Span::current().record("message", &tracing::field::debug(&msg));

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                self.process_message(text.trim(), ctx);
            }
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {
                tracing::info!("Invalid message");
                ctx.stop();
            }
        }
    }
}

impl Handler<ClientMessage> for WebsocketSystem {
    type Result = ();

    #[tracing::instrument(name = "Redirecting message to client", skip(self, ctx))]
    fn handle(&mut self, message: ClientMessage, ctx: &mut Self::Context) -> Self::Result {
        match serde_json::to_string(&message) {
            Ok(message) => ctx.text(message),
            Err(e) => tracing::error!("Failed to send message to client: {:?}", e),
        }
    }
}
