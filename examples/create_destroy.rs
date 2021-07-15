
use yajnir::jvm::{JavaVM, JniVersion, VmOptions, VmError};

fn main() -> Result<(), VmError> {
	let options = VmOptions::new(JniVersion::V10);
	let (vm, _env) = JavaVM::create(options).expect("error creating VM");

	/* actions that require a JavaVM or JniEnv */

	vm.destroy().expect("error destroying vm");
	
	eprintln!("Finished");
	Ok(())
}
