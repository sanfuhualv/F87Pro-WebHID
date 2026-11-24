use std::{mem::size_of, ptr};
use windows::{
    core::PCWSTR,
    Win32::{
        Devices::{DeviceAndDriverInstallation::*, HumanInterfaceDevice::*},
        Foundation::{CloseHandle, ERROR_IO_PENDING, HANDLE, WAIT_OBJECT_0},
        Storage::FileSystem::*,
        System::{Threading::*, IO::*},
    },
};

pub struct Device {
    handle: HANDLE,
    reader: Reader,
}
struct Reader {
    event: Handle,
    overlapped: Box<OVERLAPPED>,
    buffer: Box<[u8; 20]>,
    pending: bool,
}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            if self.reader.pending {
                let _ = CancelIoEx(self.handle, Some(self.reader.overlapped.as_ref()));
                let mut ignored = 0;
                let _ = GetOverlappedResult(
                    self.handle,
                    self.reader.overlapped.as_ref(),
                    &mut ignored,
                    true,
                );
            }
            let _ = CloseHandle(self.handle);
        }
    }
}
struct DeviceSet(HDEVINFO);
impl Drop for DeviceSet {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

impl Device {
    pub fn open() -> Result<Self, String> {
        unsafe {
            let guid = HidD_GetHidGuid();
            let set = DeviceSet(
                SetupDiGetClassDevsW(
                    Some(&guid),
                    PCWSTR::null(),
                    None,
                    DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
                )
                .map_err(|e| e.to_string())?,
            );
            let mut index = 0;
            loop {
                let mut interface = SP_DEVICE_INTERFACE_DATA {
                    cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                    ..Default::default()
                };
                if SetupDiEnumDeviceInterfaces(set.0, None, &guid, index, &mut interface).is_err() {
                    break;
                }
                index += 1;
                let mut needed = 0;
                let _ = SetupDiGetDeviceInterfaceDetailW(
                    set.0,
                    &interface,
                    None,
                    0,
                    Some(&mut needed),
                    None,
                );
                if !(8..=65536).contains(&needed) {
                    continue;
                }
                let mut storage = vec![0u64; (needed as usize).div_ceil(8)];
                let detail = storage.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
                (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
                if SetupDiGetDeviceInterfaceDetailW(
                    set.0,
                    &interface,
                    Some(detail),
                    needed,
                    None,
                    None,
                )
                .is_err()
                {
                    continue;
                }
                let path = PCWSTR(ptr::addr_of!((*detail).DevicePath).cast());
                let Ok(h) = CreateFileW(
                    path,
                    0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                ) else {
                    continue;
                };
                let metadata = Handle(h);
                let mut attr = HIDD_ATTRIBUTES {
                    Size: size_of::<HIDD_ATTRIBUTES>() as u32,
                    ..Default::default()
                };
                if !HidD_GetAttributes(metadata.0, &mut attr)
                    || attr.VendorID != 0x3554
                    || attr.ProductID != 0xfa09
                {
                    continue;
                }
                let mut data = PHIDP_PREPARSED_DATA::default();
                if !HidD_GetPreparsedData(metadata.0, &mut data) {
                    continue;
                }
                let mut caps = HIDP_CAPS::default();
                let status = HidP_GetCaps(data, &mut caps);
                let _ = HidD_FreePreparsedData(data);
                if status.0 < 0 || caps.UsagePage != 0xff02 || caps.Usage != 2 {
                    continue;
                }
                if caps.OutputReportByteLength != 20 || caps.InputReportByteLength != 20 {
                    return Err(format!(
                        "接收器报告长度为 {}，预期 20；已拒绝发送",
                        caps.OutputReportByteLength
                    ));
                }
                let handle = CreateFileW(
                    path,
                    0xc0000000,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_FLAG_OVERLAPPED,
                    None,
                )
                .map_err(|e| format!("无法打开接收器，请关闭官方驱动及网页连接：{e}"))?;
                let event = match CreateEventW(None, true, false, None) {
                    Ok(e) => Handle(e),
                    Err(e) => {
                        let _ = CloseHandle(handle);
                        return Err(e.to_string());
                    }
                };
                let overlapped = Box::new(OVERLAPPED {
                    hEvent: event.0,
                    ..Default::default()
                });
                let _ = HidD_SetNumInputBuffers(handle, 256);
                return Ok(Self {
                    handle,
                    reader: Reader {
                        event,
                        overlapped,
                        buffer: Box::new([0; 20]),
                        pending: false,
                    },
                });
            }
            Err("等待 F87 PRO 2.4G 接收器（3554:FA09）".into())
        }
    }
    pub fn send(&self, report: &[u8; 20]) -> Result<(), String> {
        self.send_with_timeout(report, 500)
    }
    pub fn send_realtime(&self, report: &[u8; 20], timeout_ms: u32) -> Result<(), String> {
        self.send_with_timeout(report, timeout_ms.clamp(1, 40))
    }
    fn send_with_timeout(&self, report: &[u8; 20], timeout_ms: u32) -> Result<(), String> {
        unsafe {
            let event = Handle(CreateEventW(None, true, false, None).map_err(|e| e.to_string())?);
            let mut overlapped = OVERLAPPED {
                hEvent: event.0,
                ..Default::default()
            };
            if let Err(e) = WriteFile(self.handle, Some(report), None, Some(&mut overlapped)) {
                if e.code() != windows::core::HRESULT::from_win32(ERROR_IO_PENDING.0) {
                    return Err(e.to_string());
                }
            }
            if WaitForSingleObject(event.0, timeout_ms) != WAIT_OBJECT_0 {
                let _ = CancelIoEx(self.handle, Some(&overlapped));
                let mut ignored = 0;
                let _ = GetOverlappedResult(self.handle, &overlapped, &mut ignored, true);
                return Err("HID 写入超时，等待重新连接".into());
            }
            let mut written = 0;
            GetOverlappedResult(self.handle, &overlapped, &mut written, false)
                .map_err(|e| e.to_string())?;
            if written != 20 {
                return Err(format!("HID 报告未完整写入：{written}/20"));
            }
            Ok(())
        }
    }
    pub fn poll(&mut self) -> Result<Option<[u8; 20]>, String> {
        unsafe {
            let r = &mut self.reader;
            if !r.pending {
                ResetEvent(r.event.0).map_err(|e| e.to_string())?;
                *r.overlapped = OVERLAPPED {
                    hEvent: r.event.0,
                    ..Default::default()
                };
                if let Err(e) = ReadFile(
                    self.handle,
                    Some(r.buffer.as_mut()),
                    None,
                    Some(r.overlapped.as_mut()),
                ) {
                    if e.code() != windows::core::HRESULT::from_win32(ERROR_IO_PENDING.0) {
                        return Err(e.to_string());
                    }
                }
                r.pending = true;
            }
            if WaitForSingleObject(r.event.0, 0) != WAIT_OBJECT_0 {
                return Ok(None);
            }
            let mut n = 0;
            let result = GetOverlappedResult(self.handle, r.overlapped.as_ref(), &mut n, false);
            r.pending = false;
            result.map_err(|e| e.to_string())?;
            if n != 20 {
                return Err(format!("HID 输入长度异常：{n}/20"));
            }
            Ok(Some(*r.buffer))
        }
    }
}
