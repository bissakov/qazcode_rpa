#include "element.h"
#include "utils.h"
#include <windows.h>
#include <uiautomation.h>
#include <stdlib.h>
#include <string.h>

ElementHandle* element_find_by_name(const char* name, int timeout_ms) {
    if (!name || !g_uia) return NULL;

    IUIAutomationElement* root = NULL;
    HRESULT hr = g_uia->lpVtbl->GetRootElement(g_uia, &root);
    if (FAILED(hr) || !root) return NULL;

    IUIAutomationCondition* condition = NULL;
    VARIANT var;
    VariantInit(&var);
    var.vt = VT_BSTR;

    int name_len = MultiByteToWideChar(CP_UTF8, 0, name, -1, NULL, 0);
    wchar_t* wide_name = (wchar_t*)malloc(name_len * sizeof(wchar_t));
    if (!wide_name) {
        root->lpVtbl->Release(root);
        return NULL;
    }
    MultiByteToWideChar(CP_UTF8, 0, name, -1, wide_name, name_len);

    var.bstrVal = SysAllocString(wide_name);
    free(wide_name);

    hr = g_uia->lpVtbl->CreatePropertyCondition(g_uia, UIA_NamePropertyId, var, &condition);
    VariantClear(&var);

    if (FAILED(hr) || !condition) {
        root->lpVtbl->Release(root);
        return NULL;
    }

    IUIAutomationElement* element = NULL;
    hr = root->lpVtbl->FindFirst(root, TreeScope_Descendants, condition, &element);

    condition->lpVtbl->Release(condition);
    root->lpVtbl->Release(root);

    if (FAILED(hr) || !element) return NULL;

    ElementHandle* handle = (ElementHandle*)malloc(sizeof(ElementHandle));
    if (!handle) {
        element->lpVtbl->Release(element);
        return NULL;
    }

    handle->handle = (void*)element;
    return handle;
}

ElementHandle* element_find_by_automation_id(const char* id, int timeout_ms) {
    if (!id || !g_uia) return NULL;

    IUIAutomationElement* root = NULL;
    HRESULT hr = g_uia->lpVtbl->GetRootElement(g_uia, &root);
    if (FAILED(hr) || !root) return NULL;

    IUIAutomationCondition* condition = NULL;
    VARIANT var;
    VariantInit(&var);
    var.vt = VT_BSTR;

    int id_len = MultiByteToWideChar(CP_UTF8, 0, id, -1, NULL, 0);
    wchar_t* wide_id = (wchar_t*)malloc(id_len * sizeof(wchar_t));
    if (!wide_id) {
        root->lpVtbl->Release(root);
        return NULL;
    }
    MultiByteToWideChar(CP_UTF8, 0, id, -1, wide_id, id_len);

    var.bstrVal = SysAllocString(wide_id);
    free(wide_id);

    hr = g_uia->lpVtbl->CreatePropertyCondition(g_uia, UIA_AutomationIdPropertyId, var, &condition);
    VariantClear(&var);

    if (FAILED(hr) || !condition) {
        root->lpVtbl->Release(root);
        return NULL;
    }

    IUIAutomationElement* element = NULL;
    hr = root->lpVtbl->FindFirst(root, TreeScope_Descendants, condition, &element);

    condition->lpVtbl->Release(condition);
    root->lpVtbl->Release(root);

    if (FAILED(hr) || !element) return NULL;

    ElementHandle* handle = (ElementHandle*)malloc(sizeof(ElementHandle));
    if (!handle) {
        element->lpVtbl->Release(element);
        return NULL;
    }

    handle->handle = (void*)element;
    return handle;
}

ElementHandle* element_find_by_class_name(const char* class_name, int timeout_ms) {
    if (!class_name || !g_uia) return NULL;

    IUIAutomationElement* root = NULL;
    HRESULT hr = g_uia->lpVtbl->GetRootElement(g_uia, &root);
    if (FAILED(hr) || !root) return NULL;

    IUIAutomationCondition* condition = NULL;
    VARIANT var;
    VariantInit(&var);
    var.vt = VT_BSTR;

    int class_len = MultiByteToWideChar(CP_UTF8, 0, class_name, -1, NULL, 0);
    wchar_t* wide_class = (wchar_t*)malloc(class_len * sizeof(wchar_t));
    if (!wide_class) {
        root->lpVtbl->Release(root);
        return NULL;
    }
    MultiByteToWideChar(CP_UTF8, 0, class_name, -1, wide_class, class_len);

    var.bstrVal = SysAllocString(wide_class);
    free(wide_class);

    hr = g_uia->lpVtbl->CreatePropertyCondition(g_uia, UIA_ClassNamePropertyId, var, &condition);
    VariantClear(&var);

    if (FAILED(hr) || !condition) {
        root->lpVtbl->Release(root);
        return NULL;
    }

    IUIAutomationElement* element = NULL;
    hr = root->lpVtbl->FindFirst(root, TreeScope_Descendants, condition, &element);

    condition->lpVtbl->Release(condition);
    root->lpVtbl->Release(root);

    if (FAILED(hr) || !element) return NULL;

    ElementHandle* handle = (ElementHandle*)malloc(sizeof(ElementHandle));
    if (!handle) {
        element->lpVtbl->Release(element);
        return NULL;
    }

    handle->handle = (void*)element;
    return handle;
}

ElementHandle** element_get_children(const ElementHandle* element, int* count) {
    if (!element || !count || !g_uia) return NULL;

    IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
    if (!elem) return NULL;

    IUIAutomationTreeWalker* walker = NULL;
    HRESULT hr = g_uia->lpVtbl->get_ControlViewWalker(g_uia, &walker);
    if (FAILED(hr) || !walker) return NULL;

    IUIAutomationElement* child = NULL;
    hr = walker->lpVtbl->GetFirstChildElement(walker, elem, &child);

    int capacity = 16;
    int child_count = 0;
    ElementHandle** children = (ElementHandle**)malloc(capacity * sizeof(ElementHandle*));
    if (!children) {
        walker->lpVtbl->Release(walker);
        return NULL;
    }

    while (SUCCEEDED(hr) && child) {
        if (child_count >= capacity) {
            capacity *= 2;
            ElementHandle** new_children = (ElementHandle**)realloc(
                children,
                capacity * sizeof(ElementHandle*)
            );
            if (!new_children) {
                for (int i = 0; i < child_count; i++) {
                    ((IUIAutomationElement*)children[i]->handle)->lpVtbl->Release(
                        (IUIAutomationElement*)children[i]->handle
                    );
                    free(children[i]);
                }
                free(children);
                child->lpVtbl->Release(child);
                walker->lpVtbl->Release(walker);
                return NULL;
            }
            children = new_children;
        }

        ElementHandle* handle = (ElementHandle*)malloc(sizeof(ElementHandle));
        if (!handle) break;

        handle->handle = (void*)child;
        children[child_count++] = handle;

        IUIAutomationElement* next = NULL;
        hr = walker->lpVtbl->GetNextSiblingElement(walker, child, &next);
        child = next;
    }

    walker->lpVtbl->Release(walker);

    *count = child_count;
    return children;
}

ElementHandle* element_get_parent(const ElementHandle* element) {
    if (!element || !g_uia) return NULL;

    IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
    if (!elem) return NULL;

    IUIAutomationTreeWalker* walker = NULL;
    HRESULT hr = g_uia->lpVtbl->get_ControlViewWalker(g_uia, &walker);
    if (FAILED(hr) || !walker) return NULL;

    IUIAutomationElement* parent = NULL;
    hr = walker->lpVtbl->GetParentElement(walker, elem, &parent);
    walker->lpVtbl->Release(walker);

    if (FAILED(hr) || !parent) return NULL;

    ElementHandle* handle = (ElementHandle*)malloc(sizeof(ElementHandle));
    if (!handle) {
        parent->lpVtbl->Release(parent);
        return NULL;
    }

    handle->handle = (void*)parent;
    return handle;
}

int element_get_text(const ElementHandle* element, char* buffer, int buffer_size) {
    if (!element || !buffer) return ERROR_NULL_POINTER;

    IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
    if (!elem) return ERROR_INVALID_HANDLE;

    BSTR name = NULL;
    HRESULT hr = elem->lpVtbl->get_CurrentName(elem, &name);
    if (FAILED(hr) || !name) return ERROR_OPERATION_FAILED;

    int len = WideCharToMultiByte(CP_UTF8, 0, name, -1, buffer, buffer_size, NULL, NULL);
    SysFreeString(name);

    return len > 0 ? SUCCESS : ERROR_OPERATION_FAILED;
}

int element_set_text(const ElementHandle* element, const char* text) {
    if (!element || !text) return ERROR_NULL_POINTER;

    IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
    if (!elem) return ERROR_INVALID_HANDLE;

    IUIAutomationValuePattern* value_pattern = NULL;
    HRESULT hr = elem->lpVtbl->GetCurrentPatternAs(
        elem,
        UIA_ValuePatternId,
        &IID_IUIAutomationValuePattern,
        (void**)&value_pattern
    );

    if (FAILED(hr) || !value_pattern) return ERROR_OPERATION_FAILED;

    int text_len = MultiByteToWideChar(CP_UTF8, 0, text, -1, NULL, 0);
    wchar_t* wide_text = (wchar_t*)malloc(text_len * sizeof(wchar_t));
    if (!wide_text) {
        value_pattern->lpVtbl->Release(value_pattern);
        return ERROR_OPERATION_FAILED;
    }
    MultiByteToWideChar(CP_UTF8, 0, text, -1, wide_text, text_len);

    BSTR bstr_text = SysAllocString(wide_text);
    free(wide_text);

    hr = value_pattern->lpVtbl->SetValue(value_pattern, bstr_text);
    SysFreeString(bstr_text);
    value_pattern->lpVtbl->Release(value_pattern);

    return SUCCEEDED(hr) ? SUCCESS : ERROR_OPERATION_FAILED;
}

int element_click(const ElementHandle* element) {
    if (!element) return ERROR_NULL_POINTER;

    IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
    if (!elem) return ERROR_INVALID_HANDLE;

    RECT rect;
    HRESULT hr = elem->lpVtbl->get_CurrentBoundingRectangle(elem, &rect);
    if (FAILED(hr)) return ERROR_OPERATION_FAILED;

    int x = (rect.left + rect.right) / 2;
    int y = (rect.top + rect.bottom) / 2;

    SetCursorPos(x, y);
    Sleep(10);

    INPUT input = {0};
    input.type = INPUT_MOUSE;
    input.mi.dwFlags = MOUSEEVENTF_LEFTDOWN;
    SendInput(1, &input, sizeof(INPUT));

    Sleep(10);

    input.mi.dwFlags = MOUSEEVENTF_LEFTUP;
    SendInput(1, &input, sizeof(INPUT));

    return SUCCESS;
}

int element_invoke(const ElementHandle* element) {
    if (!element) return ERROR_NULL_POINTER;

    IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
    if (!elem) return ERROR_INVALID_HANDLE;

    IUIAutomationInvokePattern* invoke_pattern = NULL;
    HRESULT hr = elem->lpVtbl->GetCurrentPatternAs(
        elem,
        UIA_InvokePatternId,
        &IID_IUIAutomationInvokePattern,
        (void**)&invoke_pattern
    );

    if (FAILED(hr) || !invoke_pattern) return ERROR_OPERATION_FAILED;

    hr = invoke_pattern->lpVtbl->Invoke(invoke_pattern);
    invoke_pattern->lpVtbl->Release(invoke_pattern);

    return SUCCEEDED(hr) ? SUCCESS : ERROR_OPERATION_FAILED;
}

int element_get_rect(const ElementHandle* element, Rect* rect) {
    if (!element || !rect) return ERROR_NULL_POINTER;

    IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
    if (!elem) return ERROR_INVALID_HANDLE;

    RECT win_rect;
    HRESULT hr = elem->lpVtbl->get_CurrentBoundingRectangle(elem, &win_rect);
    if (FAILED(hr)) return ERROR_OPERATION_FAILED;

    rect->x = win_rect.left;
    rect->y = win_rect.top;
    rect->width = win_rect.right - win_rect.left;
    rect->height = win_rect.bottom - win_rect.top;

    return SUCCESS;
}

int element_is_enabled(const ElementHandle* element) {
    if (!element) return 0;

    IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
    if (!elem) return 0;

    BOOL enabled = FALSE;
    HRESULT hr = elem->lpVtbl->get_CurrentIsEnabled(elem, &enabled);

    return (SUCCEEDED(hr) && enabled) ? 1 : 0;
}

void element_free(ElementHandle* element) {
    if (element) {
        if (element->handle) {
            IUIAutomationElement* elem = (IUIAutomationElement*)element->handle;
            elem->lpVtbl->Release(elem);
        }
        free(element);
    }
}
