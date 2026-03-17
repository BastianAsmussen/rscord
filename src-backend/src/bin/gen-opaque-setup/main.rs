use opaque_ke::ServerSetup;
use rand_core_06::OsRng;
use src_backend::api::opaque::DefaultCipherSuite;

fn main() {
    let setup = ServerSetup::<DefaultCipherSuite>::new(&mut OsRng);

    println!("OPAQUE_SERVER_SETUP={}", hex::encode(setup.serialize()));
}
