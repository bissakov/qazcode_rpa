#include "window.h"
#include "utils.h"
#include <windows.h>
#include <stdlib.h>
#include <string.h>

WindowHandle* window_find_by_title(const char* title) {
    if (!title) return NULL;

    HWND hwnd = FindWindowA(NULL, title);
    if (!hwnd) return NULL;

    WindowHandle* handle = (WindowHandle*)malloc(sizeof(WindowHandle));
    if (!handle) return NULL;

    handle->handle = (void*)hwnd;
    return handle;
}

WindowHandle* window_find_by_class(const char* class_name) {
    if (!class_name) return NULL;

    HWND hwnd = FindWindowA(class_name, NULL);
    if (!hwnd) return NULL;

    WindowHandle* handle = (WindowHandle*)malloc(sizeof(WindowHandle));
    if (!handle) return NULL;

    handle->handle = (void*)hwnd;
    return handle;
}

WindowHandle* window_get_focused(void) {
    HWND hwnd = GetForegroundWindow();
    if (!hwnd) return NULL;

    WindowHandle* handle = (WindowHandle*)malloc(sizeof(WindowHandle));
    if (!handle) return NULL;

    handle->handle = (void*)hwnd;
    return handle;
}

typedef struct {
    WindowHandle** windows;
    int count;
    int capacity;
} EnumWindowsData;

static BOOL CALLBACK EnumWindowsProc(HWND hwnd, LPARAM lParam) {
    EnumWindowsData* data = (EnumWindowsData*)lParam;

    if (!IsWindowVisible(hwnd)) return TRUE;

    if (data->count >= data->capacity) {
        data->capacity *= 2;
        WindowHandle** new_windows = (WindowHandle**)realloc(
            data->windows,
            data->capacity * sizeof(WindowHandle*)
        );
        if (!new_windows) return FALSE;
        data->windows = new_windows;
    }

    WindowHandle* handle = (WindowHandle*)malloc(sizeof(WindowHandle));
    if (!handle) return FALSE;

    handle->handle = (void*)hwnd;
    data->windows[data->count++] = handle;

    return TRUE;
}

WindowHandle** window_get_all(int* count) {
    if (!count) return NULL;

    EnumWindowsData data;
    data.capacity = 32;
    data.count = 0;
    data.windows = (WindowHandle**)malloc(data.capacity * sizeof(WindowHandle*));
    if (!data.windows) return NULL;

    EnumWindows(EnumWindowsProc, (LPARAM)&data);

    *count = data.count;
    return data.windows;
}

int window_get_rect(const WindowHandle* window, Rect* rect) {
    if (!window || !rect) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    RECT win_rect;
    if (!GetWindowRect((HWND)window->handle, &win_rect)) {
        return ERROR_OPERATION_FAILED;
    }

    rect->x = win_rect.left;
    rect->y = win_rect.top;
    rect->width = win_rect.right - win_rect.left;
    rect->height = win_rect.bottom - win_rect.top;

    return SUCCESS;
}

int window_is_visible(const WindowHandle* window) {
    if (!window || !window->handle) return 0;
    return IsWindowVisible((HWND)window->handle) ? 1 : 0;
}

int window_set_focus(const WindowHandle* window) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    if (!SetForegroundWindow((HWND)window->handle)) {
        return ERROR_OPERATION_FAILED;
    }

    return SUCCESS;
}

int window_close(const WindowHandle* window) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    if (!PostMessage((HWND)window->handle, WM_CLOSE, 0, 0)) {
        return ERROR_OPERATION_FAILED;
    }

    return SUCCESS;
}

int window_maximize(const WindowHandle* window) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    if (!ShowWindow((HWND)window->handle, SW_MAXIMIZE)) {
        return ERROR_OPERATION_FAILED;
    }

    return SUCCESS;
}

int window_minimize(const WindowHandle* window) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    if (!ShowWindow((HWND)window->handle, SW_MINIMIZE)) {
        return ERROR_OPERATION_FAILED;
    }

    return SUCCESS;
}

int window_click(const WindowHandle* window, int x, int y) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    HWND hwnd = (HWND)window->handle;
    LPARAM lParam = MAKELPARAM(x, y);

    PostMessage(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, lParam);
    Sleep(10);
    PostMessage(hwnd, WM_LBUTTONUP, 0, lParam);

    return SUCCESS;
}

int window_double_click(const WindowHandle* window, int x, int y) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    HWND hwnd = (HWND)window->handle;
    LPARAM lParam = MAKELPARAM(x, y);

    PostMessage(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, lParam);
    Sleep(10);
    PostMessage(hwnd, WM_LBUTTONUP, 0, lParam);
    Sleep(10);
    PostMessage(hwnd, WM_LBUTTONDBLCLK, MK_LBUTTON, lParam);
    Sleep(10);
    PostMessage(hwnd, WM_LBUTTONUP, 0, lParam);

    return SUCCESS;
}

int window_right_click(const WindowHandle* window, int x, int y) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    HWND hwnd = (HWND)window->handle;
    LPARAM lParam = MAKELPARAM(x, y);

    PostMessage(hwnd, WM_RBUTTONDOWN, MK_RBUTTON, lParam);
    Sleep(10);
    PostMessage(hwnd, WM_RBUTTONUP, 0, lParam);

    return SUCCESS;
}

int window_type_text(const WindowHandle* window, const char* text) {
    if (!window || !text) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    HWND hwnd = (HWND)window->handle;

    while (*text) {
        PostMessage(hwnd, WM_CHAR, (WPARAM)*text, 0);
        text++;
        Sleep(5);
    }

    return SUCCESS;
}

int window_key_down(const WindowHandle* window, int key) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    PostMessage((HWND)window->handle, WM_KEYDOWN, (WPARAM)key, 0);
    return SUCCESS;
}

int window_key_up(const WindowHandle* window, int key) {
    if (!window) return ERROR_NULL_POINTER;
    if (!window->handle) return ERROR_INVALID_HANDLE;

    PostMessage((HWND)window->handle, WM_KEYUP, (WPARAM)key, 0);
    return SUCCESS;
}

void window_free(WindowHandle* window) {
    if (window) {
        free(window);
    }
}
