#import <AppKit/AppKit.h>
#import <objc/runtime.h>
#include <stdlib.h>
#include <string.h>

extern void arctis_dock_wake(void);
static NSMenu *recentMenu;
static char *selection;

@interface ArctisDockActions : NSObject
- (void)openRecent:(id)sender;
@end
@implementation ArctisDockActions
- (void)openRecent:(id)sender {
    NSString *path = [sender representedObject];
    free(selection);
    selection = strdup([path fileSystemRepresentation]);
    arctis_dock_wake();
}
@end
static ArctisDockActions *actions;
static NSMenu *dockMenu(id self, SEL cmd, NSApplication *app) {
    (void)self; (void)cmd; (void)app;
    return recentMenu;
}
void arctis_dock_init(void) {
    static BOOL installed = NO;
    if (installed) return;
    id delegate = [NSApp delegate];
    if (!delegate) return;
    Class oldClass = object_getClass(delegate);
    Class sub = objc_allocateClassPair(oldClass, "ArctisDockAppDelegate", 0);
    if (!sub) sub = objc_getClass("ArctisDockAppDelegate");
    else {
        class_addMethod(sub, @selector(applicationDockMenu:), (IMP)dockMenu, "@@:@");
        objc_registerClassPair(sub);
    }
    if (!sub) return;
    [delegate retain];
    [NSApp setDelegate:nil];
    object_setClass(delegate, sub);
    [NSApp setDelegate:delegate];
    [delegate release];
    actions = [[ArctisDockActions alloc] init];
    recentMenu = [[NSMenu alloc] initWithTitle:@"Recent Projects"];
    installed = YES;
}
void arctis_dock_set_history(const char *const *paths, size_t count) {
    if (!recentMenu) return;
    [recentMenu removeAllItems];
    for (size_t i = 0; i < count; ++i) {
        NSString *path = [NSString stringWithUTF8String:paths[i]];
        if (!path) continue;
        NSString *label = [path lastPathComponent];
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:label action:@selector(openRecent:) keyEquivalent:@""];
        [item setRepresentedObject:path];
        [item setToolTip:path];
        [item setTarget:actions];
        [recentMenu addItem:item];
        [item release];
    }
    if (count == 0) {
        NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:@"No recent projects" action:nil keyEquivalent:@""];
        [item setEnabled:NO]; [recentMenu addItem:item]; [item release];
    }
}
const char *arctis_dock_take_selection(void) {
    static char *returned;
    free(returned); returned = selection; selection = NULL; return returned;
}
