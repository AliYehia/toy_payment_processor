use std::collections::HashMap;

pub struct Client {
    pub id: u16,
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,
}

impl Client {
    pub fn new(id: u16) -> Client {
        Client {
            id,
            available: 0.0,
            held: 0.0,
            total: 0.0,
            locked: false,
        }
    }
}

pub struct Clients  {
    pub clients: HashMap<u16, Client>,
}

impl Clients {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn add_client(&mut self, client_id: u16) -> &mut Client {
        self.clients.entry(client_id).or_insert_with(|| Client::new(client_id))
    }

    pub fn find_client(&mut self, client_id: u16) -> Option<&mut Client> {
        self.clients.get_mut(&client_id)
    }
}
