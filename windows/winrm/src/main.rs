use clap::{Arg, ArgAction, command};
use std::collections::HashMap;
use windows::core::{BSTR, Error, HSTRING};
use windows::{
    Data::Xml::Dom::{XmlDocument, XmlElement},
    Foundation::IReference,
    Win32::System::{
        Com::{CLSCTX_ALL, CoCreateInstance, CoInitialize},
        RemoteManagement::{IWSManEnumerator, IWSManEx3, IWSManSession, WSMan},
        Variant::VARIANT,
    },
    core::Interface,
};

// https://learn.microsoft.com/en-us/windows/win32/winrm/installation-and-configuration-for-windows-remote-management
// 「識別されていないネットワーク」のネットワークプロファイルはローカルセキュリティポリシーで変更する。
//  「セキュリティの設定」-「ネットワークリストマネージャポリシー」-「識別されていないネットワーク」
fn main() -> Result<(), Error> {
    let matches = command!()
        .arg(Arg::new("class").required(true))
        .arg(
            Arg::new("host")
                .short('r')
                .long("host")
                .default_value("http://127.0.0.1:5985"),
        )
        .arg(
            Arg::new("namespace")
                .short('n')
                .long("namespace")
                .default_value("root/cimv2"),
        )
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .arg(Arg::new("yaml").long("yaml").action(ArgAction::SetTrue))
        .get_matches();

    let class = matches.get_one::<String>("class").expect("required");
    let host = matches.get_one::<String>("host").expect("defaulted");
    let namespace = matches.get_one::<String>("namespace").expect("defaulted");
    let json = matches.get_flag("json");
    let yaml = matches.get_flag("yaml");

    // https://learn.microsoft.com/en-us/windows/win32/api/objbase/nf-objbase-coinitialize
    unsafe { CoInitialize(None).ok()? };

    // https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cocreateinstance
    let wsman: IWSManEx3 = unsafe { CoCreateInstance(&WSMan, None, CLSCTX_ALL)? };

    // https://learn.microsoft.com/en-us/windows/win32/winrm/wsman-createconnectionoptions
    //let options = unsafe { wsman.CreateConnectionOptions()? };

    // https://learn.microsoft.com/en-us/windows/win32/winrm/wsman-createsession
    let session = unsafe { wsman.CreateSession(&BSTR::from(host), 0, None)? };
    let session: IWSManSession = session.cast()?;

    // https://learn.microsoft.com/en-us/windows/win32/api/wsmandisp/nf-wsmandisp-iwsmansession-enumerate
    let resource_uri = format!("http://schemas.microsoft.com/wbem/wsman/1/wmi/{namespace}/{class}");
    let resource_uri = VARIANT::from(resource_uri.as_str());
    let enumerator =
        unsafe { session.Enumerate(&resource_uri, &BSTR::from(""), &BSTR::from(""), 0)? };
    let enumerator: IWSManEnumerator = enumerator.cast()?;

    // https://learn.microsoft.com/en-us/windows/win32/api/wsmandisp/nn-wsmandisp-iwsmanenumerator
    let mut objs: Vec<HashMap<String, String>> = vec![];
    unsafe {
        let mut at_end = enumerator.AtEndOfStream()?;
        while !at_end.as_bool() {
            let item = enumerator.ReadItem()?;
            let xml = item.to_string();

            let doc = XmlDocument::new()?;
            doc.LoadXml(&HSTRING::from(xml))?;

            let root = doc.DocumentElement()?;
            let mut obj = HashMap::new();
            for child in root.ChildNodes()? {
                let node: XmlElement = child.cast()?;
                let name = node.LocalName()?;
                let name: IReference<HSTRING> = name.cast()?;
                let value = node.InnerText()?;
                obj.insert(name.GetString()?.to_string_lossy(), value.to_string_lossy());
            }

            objs.push(obj);

            at_end = enumerator.AtEndOfStream()?;
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&objs).unwrap());
    } else if yaml {
        println!("{}", serde_yaml::to_string(&objs).unwrap());
    } else {
        println!("{}", serde_json::to_string_pretty(&objs).unwrap());
    }
    Ok(())
}
