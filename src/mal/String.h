/* SPDX-License-Identifier: MPL-2.0 */
/* Copyright © 2020 Fragcolor Pte. Ltd. */
/* Copyright (C) 2015 Joel Martin <github@martintribe.org> */

#ifndef INCLUDE_STRING_H
#define INCLUDE_STRING_H

#include <string>
#include <vector>

typedef std::string String;
typedef std::vector<String> StringVec;

#define STRF stringPrintf
#define PLURAL(n) &("s"[(n) == 1])

extern String stringPrintf(const char *fmt, ...);
extern String copyAndFree(char *mallocedString);
extern String escape(const String &s);
extern String unescape(const String &s);

#endif // INCLUDE_STRING_H
