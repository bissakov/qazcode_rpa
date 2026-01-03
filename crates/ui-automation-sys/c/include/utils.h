#ifndef UTILS_H
#define UTILS_H

#ifdef __cplusplus
extern "C" {
#endif

#define SUCCESS 0
#define ERROR_WINDOW_NOT_FOUND -1
#define ERROR_ELEMENT_NOT_FOUND -2
#define ERROR_INVALID_HANDLE -3
#define ERROR_OPERATION_FAILED -4
#define ERROR_TIMEOUT -5
#define ERROR_NULL_POINTER -6

typedef struct IUIAutomation IUIAutomation;
extern IUIAutomation* g_uia;

int init_uia(void);
void cleanup_uia(void);

#ifdef __cplusplus
}
#endif

#endif
