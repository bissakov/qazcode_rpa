#ifndef WINDOW_H
#define WINDOW_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    void* handle;
} WindowHandle;

typedef struct {
    int x;
    int y;
    int width;
    int height;
} Rect;

WindowHandle* window_find_by_title(const char* title);
WindowHandle* window_find_by_class(const char* class_name);
WindowHandle* window_get_focused(void);
WindowHandle** window_get_all(int* count);
int window_get_rect(const WindowHandle* window, Rect* rect);
int window_is_visible(const WindowHandle* window);
int window_set_focus(const WindowHandle* window);
int window_close(const WindowHandle* window);
int window_maximize(const WindowHandle* window);
int window_minimize(const WindowHandle* window);
int window_click(const WindowHandle* window, int x, int y);
int window_double_click(const WindowHandle* window, int x, int y);
int window_right_click(const WindowHandle* window, int x, int y);
int window_type_text(const WindowHandle* window, const char* text);
int window_key_down(const WindowHandle* window, int key);
int window_key_up(const WindowHandle* window, int key);
void window_free(WindowHandle* window);

#ifdef __cplusplus
}
#endif

#endif
