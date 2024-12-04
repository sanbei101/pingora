use async_trait::async_trait;
use pingora::prelude::*;
use std::sync::Arc;

pub struct SimpleProxy {
    pub listen: u16,                   // 监听的端口号
    pub target_host: String,           // 目标主机名
    pub tls: bool,                     // 是否是 TLS 连接
    lb: Arc<LoadBalancer<RoundRobin>>, // 负载均衡器
}

#[async_trait]
impl ProxyHttp for SimpleProxy {
    type CTX = ();

    fn new_ctx(&self) -> () {
        ()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let upstream = self.lb.select(b"", 256).unwrap();
        println!("upstream peer is: {upstream:?}");

        let peer = Box::new(HttpPeer::new(
            upstream,
            self.tls,
            self.target_host.to_owned(),
        ));
        Ok(peer)
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()> {
        upstream_request
            .insert_header("Host", &self.target_host)
            .unwrap();
        Ok(())
    }
}

impl SimpleProxy {
    pub fn new(listen: u16, upstreams: Vec<String>, target_host: String, tls: bool) -> Self {
        let lb = Arc::new(LoadBalancer::try_from_iter(upstreams).unwrap());

        Self {
            listen,
            lb,
            target_host,
            tls,
        }
    }

    pub fn as_service(self, server: &mut Server) {
        let addr = format!("0.0.0.0:{}", self.listen);
        let mut service = http_proxy_service(&server.configuration, self);
        service.add_tcp(&addr);

        server.add_service(service);

        println!("SimpleProxy listening on {}", addr);
    }
}

fn main() {
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    SimpleProxy::new(
        8081,                                    // listen
        vec!["116.205.113.211:3001".to_owned()], // upstreams
        "hentioe.dev".to_owned(),                // target_host
        true,                                    // tls
    )
    .as_service(&mut my_server);

    my_server.run_forever();
}
