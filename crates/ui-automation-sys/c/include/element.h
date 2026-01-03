#ifndef ELEMENT_H
#define ELEMENT_H

#include "window.h"
#include <windows.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct IUIAutomationElement IUIAutomationElement;
typedef struct IUIAutomationCondition IUIAutomationCondition;
typedef struct IUIAutomationValuePattern IUIAutomationValuePattern;
typedef struct IUIAutomationInvokePattern IUIAutomationInvokePattern;
typedef struct IUIAutomationTreeWalker IUIAutomationTreeWalker;

typedef struct {
    void* handle;
} ElementHandle;

ElementHandle* element_find_by_name(const char* name, int timeout_ms);
ElementHandle* element_find_by_automation_id(const char* id, int timeout_ms);
ElementHandle* element_find_by_class_name(const char* class_name, int timeout_ms);
ElementHandle** element_get_children(const ElementHandle* element, int* count);
ElementHandle* element_get_parent(const ElementHandle* element);
int element_get_text(const ElementHandle* element, char* buffer, int buffer_size);
int element_set_text(const ElementHandle* element, const char* text);
int element_click(const ElementHandle* element);
int element_invoke(const ElementHandle* element);
int element_get_rect(const ElementHandle* element, Rect* rect);
int element_is_enabled(const ElementHandle* element);
void element_free(ElementHandle* element);

#ifdef __cplusplus
}
#endif

#endif
