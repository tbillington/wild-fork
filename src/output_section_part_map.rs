use crate::alignment::Alignment;
use crate::alignment::AlignmentMap;
use crate::output_section_id;
use crate::output_section_id::OutputSectionId;
use crate::output_section_id::OutputSections;
use crate::output_section_id::NUM_GENERATED_SECTIONS;
use crate::output_section_map::OutputSectionMap;
use std::ops::AddAssign;

/// A map from each part of each output section to some value. Different sections are split into
/// parts in different ways. Sections that come from input files are split by alignment. Some
/// sections have no splitting and some have splitting that is specific to that particular section.
/// For example the symbol table is split into local then global symbols.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct OutputSectionPartMap<T> {
    pub(crate) regular: Vec<AlignmentMap<T>>,
    pub(crate) file_headers: T,
    pub(crate) got: T,
    pub(crate) plt: T,
    pub(crate) symtab_locals: T,
    pub(crate) symtab_globals: T,
    pub(crate) symtab_strings: T,
    pub(crate) shstrtab: T,
    pub(crate) rela_plt: T,
}

impl<T: Default> OutputSectionPartMap<T> {
    pub(crate) fn with_size(size: usize) -> Self {
        let mut regular = Vec::new();
        regular.resize_with(size - NUM_GENERATED_SECTIONS, Default::default);
        Self {
            regular,
            file_headers: Default::default(),
            got: Default::default(),
            plt: Default::default(),
            symtab_locals: Default::default(),
            symtab_globals: Default::default(),
            symtab_strings: Default::default(),
            shstrtab: Default::default(),
            rela_plt: Default::default(),
        }
    }
}

impl<T> OutputSectionPartMap<T> {
    pub(crate) fn len(&self) -> usize {
        self.regular.len() + NUM_GENERATED_SECTIONS
    }
}

impl<T: Default + PartialEq> OutputSectionPartMap<T> {
    /// Iterate through all contained T, producing a new map of U from the values returned by the
    /// callback.
    pub(crate) fn map<U: Default>(
        &self,
        output_sections: &OutputSections,
        mut cb: impl FnMut(OutputSectionId, &T) -> U,
    ) -> OutputSectionPartMap<U> {
        // For now we just iterate in output order.
        // TODO: Try iterating all regular sections as a single block, since that's likely more
        // efficient.
        self.output_order_map(output_sections, |section_id, _, v| cb(section_id, v))
    }

    /// Iterate through all contained T in output order, producing a new map of U from the values
    /// returned by the callback.
    pub(crate) fn output_order_map<U: Default>(
        &self,
        output_sections: &OutputSections,
        mut cb: impl FnMut(OutputSectionId, Alignment, &T) -> U,
    ) -> OutputSectionPartMap<U> {
        let mut regular = Vec::new();
        regular.resize_with(self.regular.len(), AlignmentMap::<U>::default);
        let file_headers = cb(
            output_section_id::HEADERS,
            output_section_id::HEADERS.min_alignment(),
            &self.file_headers,
        );
        self.map_regular(output_section_id::RODATA, &mut cb, &mut regular);
        self.map_regular(output_section_id::INIT_ARRAY, &mut cb, &mut regular);
        self.map_regular(output_section_id::FINI_ARRAY, &mut cb, &mut regular);
        self.map_regular(output_section_id::PREINIT_ARRAY, &mut cb, &mut regular);
        let shstrtab = cb(
            output_section_id::SHSTRTAB,
            output_section_id::SHSTRTAB.min_alignment(),
            &self.shstrtab,
        );
        let symtab_locals = cb(
            output_section_id::SYMTAB,
            output_section_id::SYMTAB.min_alignment(),
            &self.symtab_locals,
        );
        let symtab_globals = cb(
            output_section_id::SYMTAB,
            output_section_id::SYMTAB.min_alignment(),
            &self.symtab_globals,
        );
        let symtab_strings = cb(
            output_section_id::STRTAB,
            output_section_id::STRTAB.min_alignment(),
            &self.symtab_strings,
        );
        let rela_plt = cb(
            output_section_id::RELA_PLT,
            output_section_id::RELA_PLT.min_alignment(),
            &self.rela_plt,
        );
        output_sections.ro_custom.iter().for_each(|id| {
            self.map_regular(*id, &mut cb, &mut regular);
        });
        let plt = cb(
            output_section_id::PLT,
            output_section_id::PLT.min_alignment(),
            &self.plt,
        );
        self.map_regular(output_section_id::TEXT, &mut cb, &mut regular);
        self.map_regular(output_section_id::INIT, &mut cb, &mut regular);
        self.map_regular(output_section_id::FINI, &mut cb, &mut regular);
        output_sections.exec_custom.iter().for_each(|id| {
            self.map_regular(*id, &mut cb, &mut regular);
        });
        let got = cb(
            output_section_id::GOT,
            output_section_id::GOT.min_alignment(),
            &self.got,
        );
        self.map_regular(output_section_id::DATA, &mut cb, &mut regular);
        output_sections.data_custom.iter().for_each(|id| {
            self.map_regular(*id, &mut cb, &mut regular);
        });
        self.map_regular(output_section_id::TDATA, &mut cb, &mut regular);
        self.map_regular(output_section_id::TBSS, &mut cb, &mut regular);
        self.map_regular(output_section_id::BSS, &mut cb, &mut regular);
        output_sections.bss_custom.iter().for_each(|id| {
            self.map_regular(*id, &mut cb, &mut regular);
        });

        OutputSectionPartMap {
            regular,
            file_headers,
            got,
            plt,
            symtab_locals,
            symtab_globals,
            symtab_strings,
            shstrtab,
            rela_plt,
        }
    }

    fn map_regular<U: Default>(
        &self,
        id: OutputSectionId,
        cb: &mut impl FnMut(OutputSectionId, Alignment, &T) -> U,
        out: &mut [AlignmentMap<U>],
    ) {
        let offset = id.as_usize() - NUM_GENERATED_SECTIONS;
        let alignment_map = &self.regular[offset];
        out[offset] = map_alignment_map(alignment_map, cb, id);
    }

    pub(crate) fn regular_mut(
        &mut self,
        output_section_id: OutputSectionId,
        alignment: Alignment,
    ) -> &mut T {
        &mut self.regular[output_section_id.as_usize() - NUM_GENERATED_SECTIONS][alignment]
    }

    #[allow(dead_code)]
    pub(crate) fn regular(&self, output_section_id: OutputSectionId, alignment: Alignment) -> &T {
        &self.regular[output_section_id.as_usize() - NUM_GENERATED_SECTIONS][alignment]
    }

    /// Zip mutable references to values in `self` with shared references from `other` producing a
    /// new map with the returned values. For custom sections, `other` must be a subset of `self`.
    /// Values not in `other` will not be in the returned map.
    fn mut_with_map<U, V: Default>(
        &mut self,
        other: &OutputSectionPartMap<U>,
        mut cb: impl FnMut(&mut T, &U) -> V,
    ) -> OutputSectionPartMap<V> {
        let regular = self
            .regular
            .iter_mut()
            .zip(other.regular.iter())
            .map(|(t, u)| {
                t.mut_zip(u)
                    .map(|(alignment, t, u)| (alignment, cb(t, u)))
                    .collect::<AlignmentMap<V>>()
            })
            .collect();

        OutputSectionPartMap {
            regular,
            file_headers: cb(&mut self.file_headers, &other.file_headers),
            got: cb(&mut self.got, &other.got),
            plt: cb(&mut self.plt, &other.plt),
            symtab_locals: cb(&mut self.symtab_locals, &other.symtab_locals),
            symtab_globals: cb(&mut self.symtab_globals, &other.symtab_globals),
            symtab_strings: cb(&mut self.symtab_strings, &other.symtab_strings),
            shstrtab: cb(&mut self.shstrtab, &other.shstrtab),
            rela_plt: cb(&mut self.rela_plt, &other.rela_plt),
        }
    }
}

impl<T: Default> OutputSectionPartMap<T> {
    pub(crate) fn resize(&mut self, num_sections: usize) {
        self.regular
            .resize_with(num_sections - NUM_GENERATED_SECTIONS, Default::default)
    }
}

fn map_alignment_map<T: Default + PartialEq, U: Default>(
    alignment_map: &AlignmentMap<T>,
    cb: &mut impl FnMut(OutputSectionId, Alignment, &T) -> U,
    output_section_id: OutputSectionId,
) -> AlignmentMap<U> {
    // The maximum alignment is the alignment of the first non-default bucket when iterating the
    // alignment buckets in reverse order. We cap alignment to at most this value.
    let max_alignment = alignment_map
        .iter()
        .rev()
        .find(|(_, value)| *value != &Default::default())
        .map(|(alignment, _)| alignment)
        .unwrap_or_default();
    alignment_map
        .iter()
        .rev()
        .map(|(alignment, value)| {
            (
                alignment,
                cb(
                    output_section_id,
                    max_alignment.min(alignment.max(output_section_id.min_alignment())),
                    value,
                ),
            )
        })
        .collect()
}

impl<T: Copy> OutputSectionPartMap<T> {
    /// Merges the parts of each section together.
    pub(crate) fn merge_parts<U: Default + Copy>(
        &self,
        mut cb: impl FnMut(&[T]) -> U,
    ) -> OutputSectionMap<U> {
        let mut values_out = Vec::with_capacity(NUM_GENERATED_SECTIONS + self.regular.len());
        let mut update = |output_section_id: OutputSectionId, values: &[T]| {
            debug_assert_eq!(output_section_id.as_usize(), values_out.len());
            values_out.push(cb(values));
        };
        (update)(output_section_id::HEADERS, &[self.file_headers]);
        (update)(output_section_id::SHSTRTAB, &[self.shstrtab]);
        (update)(
            output_section_id::SYMTAB,
            &[self.symtab_locals, self.symtab_globals],
        );
        (update)(output_section_id::STRTAB, &[self.symtab_strings]);
        (update)(output_section_id::GOT, &[self.got]);
        (update)(output_section_id::PLT, &[self.plt]);
        (update)(output_section_id::RELA_PLT, &[self.rela_plt]);
        values_out.extend(self.regular.iter().map(|parts| cb(parts.raw_values())));
        OutputSectionMap::from_values(values_out)
    }
}

impl<T: AddAssign + Copy + Default> OutputSectionPartMap<T> {
    pub(crate) fn merge(&mut self, rhs: &Self) {
        if self.len() < rhs.len() {
            self.resize(rhs.len());
        }
        for (left, right) in self.regular.iter_mut().zip(rhs.regular.iter()) {
            left.merge(right);
        }
        self.file_headers += rhs.file_headers;
        self.got += rhs.got;
        self.plt += rhs.plt;
        self.symtab_locals += rhs.symtab_locals;
        self.symtab_globals += rhs.symtab_globals;
        self.symtab_strings += rhs.symtab_strings;
        self.shstrtab += rhs.shstrtab;
        self.rela_plt += rhs.rela_plt;
    }
}

impl<'out> OutputSectionPartMap<&'out mut [u8]> {
    pub(crate) fn take_mut(
        &mut self,
        sizes: &OutputSectionPartMap<usize>,
    ) -> OutputSectionPartMap<&'out mut [u8]> {
        self.mut_with_map(sizes, |buffer, size| {
            crate::slice::slice_take_prefix_mut(buffer, *size)
        })
    }
}

#[test]
fn test_merge_parts() {
    let output_sections = OutputSections::for_testing();
    let all_1 = OutputSectionPartMap::<u32>::with_size(output_sections.len())
        .output_order_map(&output_sections, |_, _, _| 1);
    let sum_of_1s: OutputSectionMap<u32> = all_1.merge_parts(|values| values.iter().sum());
    sum_of_1s.for_each(|section_id, sum| {
        assert!(*sum > 0, "Expected non-zero sum for section {section_id:?}");
    });

    let mut headers_only = OutputSectionPartMap::<u32>::with_size(output_sections.len());
    headers_only.file_headers += 42;
    let merged: OutputSectionMap<u32> = headers_only.merge_parts(|values| values.iter().sum());
    assert_eq!(*merged.built_in(output_section_id::HEADERS), 42);
    assert_eq!(*merged.built_in(output_section_id::TEXT), 0);
    assert_eq!(*merged.built_in(output_section_id::BSS), 0);
}

#[test]
fn test_mut_with_map() {
    let output_sections = OutputSections::for_testing();
    let mut input1 = OutputSectionPartMap::<u32>::with_size(output_sections.len())
        .output_order_map(&output_sections, |_, _, _| 1);
    let input2 = OutputSectionPartMap::<u32>::with_size(output_sections.len())
        .output_order_map(&output_sections, |_, _, _| 2);
    let expected = OutputSectionPartMap::<u32>::with_size(output_sections.len())
        .output_order_map(&output_sections, |_, _, _| 3);
    input1.mut_with_map(&input2, |a, b| *a += *b);
    assert_eq!(input1, expected);
}

#[test]
fn test_merge() {
    let output_sections = OutputSections::for_testing();
    let mut input1 = OutputSectionPartMap::<u32>::with_size(output_sections.len())
        .output_order_map(&output_sections, |_, _, _| 1);
    let input2 = OutputSectionPartMap::<u32>::with_size(output_sections.len())
        .output_order_map(&output_sections, |_, _, _| 2);
    let expected = OutputSectionPartMap::<u32>::with_size(output_sections.len())
        .output_order_map(&output_sections, |_, _, _| 3);
    input1.merge(&input2);
    assert_eq!(input1, expected);
}

#[test]
fn test_merge_with_custom_sections() {
    let output_sections = OutputSections::for_testing();
    let mut m1 = OutputSectionPartMap::<u32>::with_size(output_sections.len());
    let mut m2 = OutputSectionPartMap::<u32>::with_size(output_sections.len());
    assert_eq!(m2.len(), output_sections.len());
    m2.resize(output_sections.len() + 2);
    m1.merge(&m2);
    assert_eq!(m1.len(), output_sections.len() + 2);
}

/// We have two functions that iterate through all sections in output order (but in different ways).
/// Neither can practically and efficiently be implemented in terms of the other. This test verifies
/// that they iterate through the sections in the same order.
#[test]
fn test_output_order_map_consistent() {
    let output_sections = output_section_id::OutputSections::for_testing();
    let mut part_map = OutputSectionPartMap::<u32>::with_size(output_sections.len());
    part_map.resize(output_sections.len());
    let mut ordering_a = Vec::new();
    part_map.output_order_map(&output_sections, |output_section_id, _, _| {
        if ordering_a.last() != Some(&output_section_id.as_usize()) {
            ordering_a.push(output_section_id.as_usize());
        }
    });
    let mut ordering_b = Vec::new();
    output_sections.sections_do(|id, _| ordering_b.push(id.as_usize()));
    assert_eq!(ordering_a, ordering_b);
}
