mod socket_client;

use std::{
    collections::hash_map,
    sync::{Arc, Mutex},
};

use orion::SocketHandle;
use socket_client::Client;
use tracing::error;

#[derive(Clone)]
pub struct ClientManager {
    client_map: Arc<Mutex<hash_map::HashMap<u32, Client>>>,
    bound_clients: Arc<Mutex<hash_map::HashMap<String, u32>>>,
    reverse_bound_clients: Arc<Mutex<hash_map::HashMap<u32, String>>>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            client_map: Arc::new(Mutex::new(hash_map::HashMap::new())),
            bound_clients: Arc::new(Mutex::new(hash_map::HashMap::new())),
            reverse_bound_clients: Arc::new(Mutex::new(hash_map::HashMap::new())),
        }
    }

    pub fn add_client(&mut self, socket_handle: SocketHandle) {
        let id = socket_handle.id();
        let client = Client::new(socket_handle);
        let mut map = self.client_map.lock().unwrap();
        if map.contains_key(&id) {
            error!("Client already exists: {}", id);
            return;
        }
        map.insert(id, client);
    }

    pub fn bind_connection(&self, uid: String, socket_id: u32) {
        if let Some(_) = self.client_map.lock().unwrap().get(&socket_id) {
            self.bound_clients
                .lock()
                .unwrap()
                .insert(uid.clone(), socket_id);

            self.reverse_bound_clients
                .lock()
                .unwrap()
                .insert(socket_id, uid);
        } else {
            error!("Failed to bind connection: socket not found {}", socket_id);
        }
    }

    pub fn remove_client(&self, id: u32) {
        if let Some(_) = self.client_map.lock().unwrap().remove(&id) {
            if let Some(uid) = self.reverse_bound_clients.lock().unwrap().remove(&id) {
                self.bound_clients.lock().unwrap().remove(&uid);
            }
        }
    }

    pub fn get_client(&self, id: u32) -> Option<Client> {
        self.client_map.lock().unwrap().get(&id).cloned()
    }

    pub fn get_client_by_uid(&self, uid: &str) -> Option<Client> {
        if let Some(socket_id) = self.bound_clients.lock().unwrap().get(uid) {
            self.client_map.lock().unwrap().get(socket_id).cloned()
        } else {
            None
        }
    }
}
