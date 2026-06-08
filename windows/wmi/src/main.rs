use clap::{Arg, ArgAction, command};
use serde::Serialize;
use std::collections::HashMap;
use windows::Win32::System::{
    Com::{
        CLSCTX_ALL, CoCreateInstance, CoInitialize, CoInitializeSecurity, EOAC_NONE,
        RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE,
    },
    Variant::VARIANT,
    Wmi::{
        IWbemLocator, WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_INFINITE,
        WbemLocator,
    },
};
use windows::core::{BSTR, Error};

fn main() -> Result<(), Error> {
    let matches = command!()
        .arg(Arg::new("query").required(true))
        .arg(
            Arg::new("namespace")
                .short('n')
                .long("namespace")
                .default_value("ROOT\\CIMV2"),
        )
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .arg(Arg::new("yaml").long("yaml").action(ArgAction::SetTrue))
        .get_matches();

    let query = matches.get_one::<String>("query").expect("required");
    let namespace = matches.get_one::<String>("namespace").expect("defaulted");
    let json = matches.get_flag("json");
    let yaml = matches.get_flag("yaml");

    // https://learn.microsoft.com/en-us/windows/win32/api/objbase/nf-objbase-coinitialize
    unsafe { CoInitialize(None).ok()? };

    // https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-coinitializesecurity
    unsafe {
        CoInitializeSecurity(
            None,
            -1,
            None,
            None,
            RPC_C_AUTHN_LEVEL_DEFAULT,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
            None,
        )?
    };

    // https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cocreateinstance
    let locator: IWbemLocator = unsafe { CoCreateInstance(&WbemLocator, None, CLSCTX_ALL)? };

    // https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemlocator-connectserver
    let services = unsafe {
        locator.ConnectServer(
            &BSTR::from(namespace), // networkresource
            &BSTR::new(),           // user
            &BSTR::new(),           // password
            &BSTR::new(),           // locale
            0,                      // security flags
            &BSTR::new(),           // authority
            None,
        )?
    };

    // https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemservices-execquery
    let enumerator = unsafe {
        services.ExecQuery(
            &BSTR::from("WQL"),
            &BSTR::from(query),
            WBEM_FLAG_RETURN_IMMEDIATELY | WBEM_FLAG_FORWARD_ONLY,
            None,
        )?
    };

    let mut objs: Vec<HashMap<String, CimType>> = vec![];
    loop {
        // https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-ienumwbemclassobject-next
        let mut svcs = [None; 1];
        let mut returned = 0;
        unsafe {
            enumerator
                .Next(WBEM_INFINITE, &mut svcs, &mut returned)
                .ok()?;
        }

        if returned == 0 {
            break;
        }

        if let Some(svc) = svcs[0].as_ref() {
            // https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-beginenumeration
            unsafe { svc.BeginEnumeration(0)? };

            let mut obj = HashMap::new();
            loop {
                // https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-next
                let mut name = BSTR::new();
                let mut val = Default::default();
                let mut ty = 0;
                let mut flavor = 0;
                unsafe {
                    svc.Next(0, &mut name, &mut val, &mut ty, &mut flavor)?;
                }

                if name.is_empty() {
                    break;
                }

                obj.insert(name.to_string(), value(&val, ty).unwrap_or(CimType::Empty));
            }

            objs.push(obj);

            // https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-endenumeration
            unsafe { svc.EndEnumeration()? };
        }
    }

    // https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-couninitialize
    // Exit code: 0xc0000005, STATUS_ACCESS_VIOLATION
    //unsafe { CoUninitialize() };

    if json {
        println!("{}", serde_json::to_string_pretty(&objs).unwrap());
    } else if yaml {
        println!("{}", serde_yaml::to_string(&objs).unwrap());
    } else {
        println!("{}", serde_json::to_string_pretty(&objs).unwrap());
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CimType {
    Empty,
    Sint8(i8),
    Uint8(u8),
    Sint16(i16),
    Uint16(u16),
    Sint32(i32),
    Uint32(u32),
    Sint64(i64),
    Uint64(u64),
    Real32(f32),
    Real64(f64),
    Boolean(bool),
    String(String),
}

fn value(value: &VARIANT, ty: i32) -> Option<CimType> {
    let v000 = unsafe { &value.Anonymous.Anonymous.Anonymous };

    // https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/ne-wbemcli-cimtype_enumeration
    match ty {
        0 => Some(CimType::Empty),                           // CIM_EMPTY
        16 => Some(CimType::Sint8(unsafe { v000.cVal })),    // CIM_SINT8
        17 => Some(CimType::Uint8(unsafe { v000.bVal })),    // CIM_UINT8
        2 => Some(CimType::Sint16(unsafe { v000.iVal })),    // CIM_SINT16
        18 => Some(CimType::Uint16(unsafe { v000.uiVal })),  // CIM_UINT16
        3 => Some(CimType::Sint32(unsafe { v000.lVal })),    // CIM_SINT32
        19 => Some(CimType::Uint32(unsafe { v000.ulVal })),  // CIM_UINT32
        20 => Some(CimType::Sint64(unsafe { v000.llVal })),  // CIM_SINT64
        21 => Some(CimType::Uint64(unsafe { v000.ullVal })), // CIM_UINT64
        4 => Some(CimType::Real32(unsafe { v000.fltVal })),  // CIM_REAL32
        5 => Some(CimType::Real64(unsafe { v000.dblVal })),  // CIM_REAL64
        11 => Some(CimType::Boolean(unsafe { v000.boolVal.as_bool() })), // CIM_BOOLEAN
        8 => Some(CimType::String(unsafe { v000.bstrVal.to_string() })), // CIM_STRING
        _ => None,
    }
}
