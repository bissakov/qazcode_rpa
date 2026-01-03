#include "utils.h"
#include <windows.h>
#include <uiautomation.h>
#include <ole2.h>

IUIAutomation* g_uia = NULL;

int init_uia(void) {
    HRESULT hr = CoInitializeEx(NULL, COINIT_MULTITHREADED);
    if (FAILED(hr) && hr != RPC_E_CHANGED_MODE) {
        return ERROR_OPERATION_FAILED;
    }

    hr = CoCreateInstance(
        &CLSID_CUIAutomation,
        NULL,
        CLSCTX_INPROC_SERVER,
        &IID_IUIAutomation,
        (void**)&g_uia
    );

    if (FAILED(hr)) {
        CoUninitialize();
        return ERROR_OPERATION_FAILED;
    }

    return SUCCESS;
}

void cleanup_uia(void) {
    if (g_uia) {
        g_uia->lpVtbl->Release(g_uia);
        g_uia = NULL;
    }
    CoUninitialize();
}
