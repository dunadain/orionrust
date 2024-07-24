pub mod socket_client;

use std::{
    collections::hash_map,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use tracing::error;

#[derive(Clone)]
pub struct ClientManager {
    client_map: Arc<Mutex<hash_map::HashMap<u32, Arc<dyn NetClient>>>>,
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

    pub fn add_client(&mut self, id: u32, client: impl NetClient + 'static) {
        let mut map = self.client_map.lock().unwrap();
        if map.contains_key(&id) {
            error!("Client already exists: {}", id);
            return;
        }
        map.insert(id, Arc::new(client));
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

    pub fn get_client(&self, id: u32) -> Option<Arc<dyn NetClient>> {
        self.client_map.lock().unwrap().get(&id).cloned()
    }

    pub fn get_client_by_uid(&self, uid: &str) -> Option<Arc<dyn NetClient>> {
        if let Some(socket_id) = self.bound_clients.lock().unwrap().get(uid) {
            self.client_map.lock().unwrap().get(socket_id).cloned()
        } else {
            None
        }
    }
}

pub trait NetClient: Send + Sync {
    fn receive_msg(self: Arc<Self>, msg: Bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_client() {
        let mut client_manager = ClientManager::new();
        let client_id = 1;
        let client = MockClient::new();
        let cm = client_manager.clone();
        assert_eq!(cm.client_map.lock().unwrap().len(), 0);

        client_manager.add_client(client_id, client.clone());

        let map = cm.client_map.lock().unwrap();
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_bind_connection() {
        let mut client_manager = ClientManager::new();
        let uid = "user1".to_string();
        let socket_id = 1;

        client_manager.add_client(socket_id, MockClient::new());

        client_manager.bind_connection(uid.clone(), socket_id);

        let bound_clients = client_manager.bound_clients.lock().unwrap();
        let reverse_bound_clients = client_manager.reverse_bound_clients.lock().unwrap();
        assert_eq!(bound_clients.len(), 1);
        assert_eq!(bound_clients.get(&uid), Some(&socket_id));
        assert_eq!(reverse_bound_clients.len(), 1);
        assert_eq!(reverse_bound_clients.get(&socket_id), Some(&uid));
    }

    #[test]
    fn test_remove_client() {
        let mut client_manager = ClientManager::new();
        let client_id = 1;
        let uid = "user2".to_string();
        let client = MockClient::new();
        client_manager.add_client(client_id, client.clone());
        client_manager.bind_connection(uid.clone(), client_id);

        client_manager.remove_client(client_id);

        let map = client_manager.client_map.lock().unwrap();
        assert_eq!(map.len(), 0);

        let bound_clients = client_manager.bound_clients.lock().unwrap();
        assert_eq!(bound_clients.len(), 0);
        assert_eq!(bound_clients.get(&uid), None);

        let reverse_bound_clients = client_manager.reverse_bound_clients.lock().unwrap();
        assert_eq!(reverse_bound_clients.len(), 0);
        assert_eq!(reverse_bound_clients.get(&client_id), None);
    }

    #[test]
    fn test_get_client() {
        let mut client_manager = ClientManager::new();
        let client_id = 1;
        let client = MockClient::new();
        client_manager.add_client(client_id, client.clone());

        let result = client_manager.get_client(client_id);
        let has = match result {
            Some(_) => true,
            None => false,
        };

        assert!(has);
    }

    #[test]
    fn test_get_client_by_uid() {
        let client_manager = ClientManager::new();
        let uid = "user1".to_string();
        let socket_id = 1;
        client_manager.bind_connection(uid.clone(), socket_id);

        let result = client_manager.get_client_by_uid(&uid);

        let has = match result {
            Some(_) => true,
            None => false,
        };
        assert!(!has);
    }

    #[derive(Clone)]
    struct MockClient;

    impl MockClient {
        fn new() -> Self {
            MockClient
        }
    }

    impl NetClient for MockClient {
        fn receive_msg(self: Arc<Self>, msg: Bytes) {
            // Mock implementation
        }
    }
}
