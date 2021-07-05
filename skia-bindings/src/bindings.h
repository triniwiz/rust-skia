#ifndef SKIA_BINDINGS_BINDINGS_H
#define SKIA_BINDINGS_BINDINGS_H

#include "include/core/SkRefCnt.h"
#include "include/core/SkString.h"

#include <vector>
#include "modules/skresources/include/SkResources.h"

template<typename T>
inline sk_sp<T> spFromConst(const T* pt) {
    return sk_sp<T>(const_cast<T*>(pt));
}

template<typename T>
inline sk_sp<T> sp(T* pt) {
    return sk_sp<T>(pt);
}

// Used in textlayout::Paragraph::findTypefaces()

struct SkStrings {
    std::vector<SkString> strings;
};

typedef SkData* (*loadSkData)(const char resource_path[], const char resource_name[]);


#endif //SKIA_BINDINGS_BINDINGS_H
