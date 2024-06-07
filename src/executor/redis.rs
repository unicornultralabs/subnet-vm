use anyhow::{anyhow, Error, Result};
use redis::{Commands as _, Connection, RedisResult};
use redis_async::client::{ConnectionBuilder, PubsubConnection};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use url::Url;


struct RedisUri {
    is_tls: bool,
    password: Option<String>,
    host: String,
    port: u16,
}

#[derive(Debug)]
pub struct RedisService {
    client: redis::Client,
    pubsub_con: PubsubConnection,
}

impl RedisService {
    pub async fn new(redis_uri: String) -> Result<Self> {
        let client = redis::Client::open(redis_uri.clone())
            .map_err(|e| anyhow!("redis: cannot open client err={}", e))?;
        _ = client
            .get_connection()
            .map_err(|e| anyhow!("redis: cannot get connection err={}", e))?;

        let conn_builder = Self::get_redis_conn_builder_from_uri(&redis_uri)?;
        let pubsub_con = conn_builder
            .pubsub_connect()
            .await
            .map_err(|e| anyhow!("create pub sub connection failed err={}", e))?;

        Ok(Self { client, pubsub_con })
    }

    fn parse_redis_uri(redis_uri: &str) -> Result<RedisUri> {
        let parsed_url = Url::parse(redis_uri)?;

        let is_tls = match parsed_url.scheme() {
            "redis" => false,
            "rediss" => true,
            unknown => {
                return Err(anyhow::anyhow!(
                    "invalid scheme, must be 'redis' or 'rediss' unknown {}",
                    unknown
                ))
            }
        };
        let password = parsed_url.password().map(|p| p.to_string());

        let host = match parsed_url.host_str() {
            Some(host) => host.to_string(),
            None => return Err(anyhow::anyhow!("parse host failed")),
        };

        let port = match parsed_url.port() {
            Some(port) => port,
            None => return Err(anyhow::anyhow!("parse port failed")),
        };

        Ok(RedisUri {
            is_tls,
            password,
            host,
            port,
        })
    }

    pub fn get_redis_conn_builder_from_uri(redis_uri: &str) -> Result<ConnectionBuilder> {
        let redis_info =
            Self::parse_redis_uri(redis_uri).map_err(|e| anyhow!("parse failed err={}", e))?;

        let mut connection_builder: ConnectionBuilder =
            ConnectionBuilder::new(redis_info.host, redis_info.port)
                .map_err(|e| anyhow!("connection build create failed err={}", e))?;

        if redis_info.is_tls {
            connection_builder.tls();
        }

        if let Some(redis_password) = redis_info.password {
            connection_builder.password(redis_password);
        }

        Ok(connection_builder)
    }

    pub fn get_pubsub_conn(self: Arc<Self>) -> PubsubConnection {
        self.pubsub_con.clone()
    }

    pub fn hset<T>(self: Arc<Self>, key: String, field: String, obj: T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;
        match conn.hset::<String, String, String, usize>(
            key,
            field,
            serde_json::to_string(&obj).unwrap(),
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("redis failed to insert err={}", e)),
        }
    }

    pub fn hget<T>(self: Arc<Self>, key: String, field: String) -> Result<T, Error>
    where
        T: Clone + DeserializeOwned,
    {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;
        let obj_str: String = conn
            .hget(key.clone(), field.clone())
            .map_err(|e| anyhow!("redis cannot get key={}:{} err={}", key, field, e))?;
        let t = serde_json::from_str::<T>(&obj_str)
            .map_err(|e| anyhow!("redis failed to decode err={}", e))?;
        Ok(t)
    }

    pub fn hgetall<T>(self: Arc<Self>, key: String) -> Result<Vec<(String, T)>, Error>
    where
        T: Clone + DeserializeOwned,
    {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;
        let result: HashMap<String, String> = conn
            .hgetall(key.clone())
            .map_err(|e| anyhow!("redis cannot get key={} err={}", key, e))?;
        let mut rs: Vec<(String, T)> = vec![];
        for (key, obj_str) in result.iter() {
            let proxy_acc = serde_json::from_str::<T>(&obj_str)
                .map_err(|e| anyhow!("redis failed to decode err={}", e))?;
            rs.push((key.clone(), proxy_acc.clone()));
        }
        Ok(rs)
    }

    pub fn hdel(self: Arc<Self>, key: String, field: String) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;
        conn.hdel(key.clone(), field.clone())
            .map_err(|e| anyhow!("redis cannot hdel key={} field={} err={}", key, field, e))?;
        Ok(())
    }

    pub fn zadd(self: Arc<Self>, key: String, score: u32, value: u32) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;
        match conn.zadd::<String, u32, u32, ()>(key, value, score) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(
                "redis failed to insert peer into peer queue err={}",
                e
            )),
        }
    }

    pub fn zrem(self: Arc<Self>, key: String, value: u32) -> Result<(), anyhow::Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;

        match conn.zrem::<String, u32, usize>(key, value) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(
                "redis failed to remove peer in peer queue err={}",
                e
            )),
        }
    }

    pub fn zsetall(self: Arc<Self>, key: String, score: u32) -> Result<(), anyhow::Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;

        let elements: Vec<(u32, u32)> = conn
            .zrange_withscores(key.clone(), 0, -1)
            .map_err(|e| anyhow!("redis failed to get sorted set err={}", e))?;

        for (value, _) in elements {
            conn.zadd::<String, u32, u32, ()>(key.clone(), value, score)
                .map_err(|e| anyhow!("redis failed to set scores err={}", e))?;
        }

        Ok(())
    }

    pub fn zgetall(self: Arc<Self>, key: String) -> Result<Vec<(u32, u32)>, Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;

        let elements: Vec<(u32, u32)> = conn
            .zrange_withscores(key.clone(), 0, -1)
            .map_err(|e| anyhow!("redis failed to get peer queue err={}", e))?;

        let mut result: Vec<(u32, u32)> = elements
            .into_iter()
            .map(|(value, score)| (value, score))
            .collect();

        result.sort_by_key(|(_value, score)| *score);

        Ok(result)
    }

    /// this function is used to delete data of given key
    pub fn del(self: Arc<Self>, key: String) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;

        conn.del(key.clone())
            .map_err(|e| anyhow!("redis failed to delete key={} err={}", key, e))
    }

    pub async fn publish(self: Arc<Self>, chan_name: String, obj_str: String) -> Result<(), Error> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| anyhow!("cannot get connection err={}", e))?;
        conn.publish(&chan_name, &obj_str)?;
        Ok(())
    }

    pub async fn get_conn(self: Arc<Self>) -> RedisResult<Connection> {
        self.client.get_connection()
    }
}

pub struct DPNRedisKey {}
impl DPNRedisKey {
    pub fn get_geo_kf(masternode_id: String, login_session_id: String) -> (String, String) {
        (
            "peer_geo".to_owned(),
            format!("{}_{}", masternode_id.clone(), login_session_id.clone()),
        )
    }

    pub fn get_balance_kf(user_addr: String) -> (String, String) {
        (
            "client_user_balance".to_owned(),
            format!("{}", user_addr),
        )
    }

    pub fn get_peer_queue_k(masternode_id: String) -> String {
        format!("peer_queue_ms#{}_", masternode_id)
    }

    pub fn get_peers_kf(masternode_id: String, ip_u32: u32) -> (String, String) {
        (format!("peers_ms#{}", masternode_id), format!("{}", ip_u32))
    }

    pub fn get_peers_chan(masternode_id: String) -> String {
        format!("peers_updated_ms#{}", masternode_id)
    }

    pub fn get_price_kf(peer_addr: String) -> (String, String) {
        ("peer_price".to_owned(), peer_addr)
    }

    pub fn get_proxy_acc_kf(id: String) -> (String, String) {
        ("proxy_acc".to_owned(), id)
    }

    pub fn get_proxy_acc_chan() -> String {
        "proxy_acc_updated".to_string()
    }

    pub fn get_price_chan() -> String {
        "price_updated".to_string()
    }
}
